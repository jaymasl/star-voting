CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE vote_state AS ENUM ('active', 'concluded');

CREATE SCHEMA active_votes;
CREATE SCHEMA archived_votes;

CREATE OR REPLACE FUNCTION validate_option_text(options text[]) 
RETURNS boolean AS $$
    SELECT NOT EXISTS (
        SELECT 1 FROM unnest(options) AS opt
        WHERE length(opt) > 40 OR length(opt) = 0
    );
$$ LANGUAGE sql IMMUTABLE;

CREATE OR REPLACE FUNCTION validate_scores(scores integer[])
RETURNS boolean AS $$
  SELECT NOT EXISTS (
      SELECT 1 FROM unnest(scores) AS score
      WHERE score < 0 OR score > 5
  );
$$ LANGUAGE sql IMMUTABLE;

CREATE OR REPLACE FUNCTION check_user_vote_limit()
RETURNS TRIGGER AS $$
BEGIN
    IF (
        SELECT COUNT(*) 
        FROM active_votes.votes 
        WHERE user_fingerprint = NEW.user_fingerprint 
        AND state = 'active'
    ) >= 30 THEN
        RAISE EXCEPTION 'User has reached the maximum limit of active votes';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE active_votes.votes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_fingerprint VARCHAR(255) NOT NULL,
    title VARCHAR(100) NOT NULL CHECK (length(trim(title)) > 0),
    description VARCHAR(500),
    state vote_state NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    voting_ends_at TIMESTAMPTZ NOT NULL CHECK (voting_ends_at > created_at),
    archived_at TIMESTAMPTZ,
    duration_hours INTEGER NOT NULL,
    duration_minutes INTEGER NOT NULL,
    options TEXT[] NOT NULL,
    CONSTRAINT valid_duration CHECK (
        duration_hours >= 0 
        AND duration_minutes BETWEEN 0 AND 59
        AND (duration_hours > 0 OR duration_minutes > 0)
        AND (duration_hours * 60 + duration_minutes) <= (30 * 24 * 60)
    ),
    CONSTRAINT valid_options CHECK (
        array_length(options, 1) BETWEEN 2 AND 20
        AND array_length(array_remove(options, NULL), 1) = array_length(options, 1)
        AND validate_option_text(options)
    ),
    CONSTRAINT valid_state_transitions CHECK (
        CASE state
            WHEN 'active' THEN archived_at IS NULL
            WHEN 'concluded' THEN archived_at IS NOT NULL
        END
    )
);

CREATE TABLE active_votes.ballots (
    id BIGSERIAL PRIMARY KEY,
    vote_id UUID NOT NULL REFERENCES active_votes.votes(id) ON DELETE CASCADE,
    user_fingerprint VARCHAR(255) NOT NULL,
    scores INTEGER[] NOT NULL,
    cast_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_voter UNIQUE (vote_id, user_fingerprint),
    CONSTRAINT valid_scores CHECK (
        array_length(scores, 1) > 0 
        AND validate_scores(scores)
    )
);

CREATE TABLE archived_votes.votes (
    id UUID PRIMARY KEY,
    user_fingerprint VARCHAR(255) NOT NULL,
    title VARCHAR(100) NOT NULL,
    description VARCHAR(500),
    created_at TIMESTAMPTZ NOT NULL,
    voting_ends_at TIMESTAMPTZ NOT NULL,
    archived_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    archive_expires_at TIMESTAMPTZ NOT NULL DEFAULT NOW() + INTERVAL '30 days',
    duration_hours INTEGER NOT NULL,
    duration_minutes INTEGER NOT NULL,
    options TEXT[] NOT NULL,
    final_stats JSONB NOT NULL,
    winner TEXT NOT NULL,
    head_to_head JSONB NOT NULL
);

CREATE TABLE archived_votes.ballots (
    id BIGINT PRIMARY KEY,
    vote_id UUID NOT NULL REFERENCES archived_votes.votes(id) ON DELETE CASCADE,
    user_fingerprint VARCHAR(255) NOT NULL,
    scores INTEGER[] NOT NULL,
    cast_at TIMESTAMPTZ NOT NULL
);

CREATE TRIGGER check_vote_limit
    BEFORE INSERT ON active_votes.votes
    FOR EACH ROW
    EXECUTE FUNCTION check_user_vote_limit();

CREATE OR REPLACE FUNCTION calculate_vote_stats(vote_id_param UUID)
RETURNS TABLE (
total_ballots BIGINT,
option_scores JSONB
    ) AS $$
    WITH ballot_counts AS (
        SELECT COUNT(*) AS total_count
        FROM active_votes.ballots
        WHERE vote_id = vote_id_param
    ),
    score_frequencies AS (
        SELECT 
            opt_text,
            b.scores[idx] AS score,
            COUNT(*) AS freq_count
        FROM active_votes.votes v
        CROSS JOIN UNNEST(v.options) WITH ORDINALITY AS opt_enum(opt_text, idx)
        LEFT JOIN active_votes.ballots b ON b.vote_id = v.id
        WHERE v.id = vote_id_param
        GROUP BY opt_text, idx, b.scores[idx]
    ),
    option_stats AS (
        SELECT 
            opt_text,
            SUM(COALESCE(score, 0) * freq_count) AS total_score,
            ROUND(AVG(COALESCE(score, 0))::numeric, 2) AS average_score,
            SUM(freq_count) AS total_votes,
            JSONB_OBJECT_AGG(
                COALESCE(score::text, '0'),
                freq_count
            ) AS frequency
        FROM score_frequencies
        GROUP BY opt_text
    )
    SELECT 
        c.total_count,
        JSONB_OBJECT_AGG(
            s.opt_text,
            JSONB_BUILD_OBJECT(
                'total_score', s.total_score,
                'average_score', s.average_score,
                'frequency', s.frequency,
                'total_votes', s.total_votes
            )
        ) AS option_scores
    FROM ballot_counts c
    CROSS JOIN option_stats s
    GROUP BY c.total_count;
$$ LANGUAGE sql STABLE;

CREATE OR REPLACE FUNCTION update_vote_states() 
RETURNS void AS $$
DECLARE
  r RECORD;
BEGIN
    FOR r IN (
        SELECT id
        FROM active_votes.votes
        WHERE state = 'active' 
        AND voting_ends_at <= NOW()
        FOR UPDATE SKIP LOCKED
    ) LOOP
        WITH vote_data AS (
            SELECT v.*, 
                    (SELECT row_to_json(stats)::jsonb 
                    FROM calculate_vote_stats(v.id) stats) as final_stats
            FROM active_votes.votes v
            WHERE v.id = r.id
        )
        UPDATE active_votes.votes
        SET state = 'concluded',
            archived_at = NOW()
        WHERE id = r.id;
        
        INSERT INTO archived_votes.votes (
            id, user_fingerprint, title, description, created_at, voting_ends_at,
            archived_at, duration_hours, duration_minutes, options, final_stats,
            winner, head_to_head
        )
        SELECT 
            id, user_fingerprint, title, description, created_at, voting_ends_at,
            archived_at, duration_hours, duration_minutes, options, final_stats,
            COALESCE(winner, '{}'::jsonb),
            COALESCE(head_to_head, '{}'::jsonb)
        FROM vote_data;

        INSERT INTO archived_votes.ballots (id, vote_id, user_fingerprint, scores, cast_at)
        SELECT id, vote_id, user_fingerprint, scores, cast_at
        FROM active_votes.ballots
        WHERE vote_id = r.id;

        DELETE FROM active_votes.votes WHERE id = r.id;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_expired_archives()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    WITH deleted AS (
        DELETE FROM archived_votes.votes
        WHERE archive_expires_at <= NOW()
        RETURNING id
    )
    SELECT COUNT(*) INTO deleted_count FROM deleted;
    
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

CREATE INDEX idx_active_votes_state ON active_votes.votes(state);
CREATE INDEX idx_active_votes_voting_ends ON active_votes.votes(voting_ends_at) WHERE state = 'active';
CREATE INDEX idx_active_ballots_vote ON active_votes.ballots(vote_id);
CREATE INDEX idx_active_votes_created ON active_votes.votes(created_at);
CREATE INDEX idx_active_votes_user ON active_votes.votes(user_fingerprint);
CREATE INDEX idx_active_ballots_user ON active_votes.ballots(user_fingerprint);
CREATE INDEX idx_ballots_cast_time ON active_votes.ballots(cast_at);

CREATE INDEX idx_archived_votes_expires ON archived_votes.votes(archive_expires_at);
CREATE INDEX idx_archived_votes_archived ON archived_votes.votes(archived_at);
CREATE INDEX idx_archived_votes_user ON archived_votes.votes(user_fingerprint);
CREATE INDEX idx_archived_ballots_vote ON archived_votes.ballots(vote_id);
CREATE INDEX idx_archived_ballots_user ON archived_votes.ballots(user_fingerprint);