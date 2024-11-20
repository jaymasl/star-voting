use backend::{
    routes::{all_options, cast_ballot, create_vote, get_csrf_token, get_result, get_vote, list_votes, AppState},
    cors::CORS,
    catchers::{bad_request, forbidden, internal_error, not_found, too_many_requests},
};
use rocket::{routes, catchers, fs::NamedFile};
use shuttle_runtime::CustomError;
use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};
use include_dir::{include_dir, Dir};
use uuid::Uuid;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

async fn check_pending_votes(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let votes = sqlx::query!(
        "SELECT id FROM active_votes.votes
         WHERE state = 'active' AND voting_ends_at <= NOW()"
    )
    .fetch_all(pool)
    .await?;

    if !votes.is_empty() {
        info!("🔍 Found {} votes to archive", votes.len());
        for vote in votes {
            match backend::processor::VoteProcessor::archive_vote(pool, vote.id).await {
                Ok(_) => info!("✓ Archived vote {}", vote.id),
                Err(e) => error!("✗ Failed to archive vote {}: {}", vote.id, e),
            }
        }
    }

    let expired_count = sqlx::query_scalar!("SELECT cleanup_expired_archives()")
        .fetch_one(pool)
        .await?;

    if let Some(count) = expired_count {
        if count > 0 {
            info!("🗑️ Removed {} expired votes", count);
        }
    }
    Ok(())
}

async fn run_cleanup_task(pool: PgPool) {
    let mut interval = interval(Duration::from_secs(60));
    info!("🧹 Cleanup service started");

    if let Err(e) = check_pending_votes(&pool).await {
        error!("Initial cleanup failed: {}", e);
    }

    loop {
        interval.tick().await;
        if let Err(e) = check_pending_votes(&pool).await {
            error!("Cleanup failed: {}", e);
        }
    }
}

#[rocket::get("/<path..>")]
async fn spa_handler(path: std::path::PathBuf, temp_dir: &rocket::State<std::path::PathBuf>) -> Option<NamedFile> {
    let file_path = temp_dir.join(&path);
    if file_path.exists() && file_path.is_file() {
        NamedFile::open(&file_path).await.ok()
    } else {
        NamedFile::open(temp_dir.join("index.html")).await.ok()
    }
}

#[shuttle_runtime::main]
async fn rocket(
    #[shuttle_shared_db::Postgres] pool: PgPool,
    #[shuttle_runtime::Secrets] secret_store: shuttle_runtime::SecretStore,
) -> shuttle_rocket::ShuttleRocket {
    info!("🚀 Starting STAR Vote server");

    let app_state = match secret_store.get("HCAPTCHA_SECRET") {
        Some(hcaptcha_secret) => {
            AppState::new_with_captcha(pool.clone(), hcaptcha_secret)
        }
        None => {
            warn!("HCAPTCHA_SECRET not found - captcha verification will be disabled");
            AppState::new(pool.clone())
        }
    };

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(CustomError::new)?;

    info!("📋 Migrations complete");

    let temp_dir = std::env::temp_dir().join(format!("star_vote_static_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");
    STATIC_DIR.extract(&temp_dir).expect("Failed to extract static files");

    tokio::spawn(run_cleanup_task(pool.clone()));

    let rocket = rocket::build()
        .attach(CORS)
        .manage(app_state)
        .manage(temp_dir.clone())
        .mount(
            "/api",
            routes![
                create_vote,
                cast_ballot,
                get_result,
                get_vote,
                list_votes,
                all_options,
                get_csrf_token
            ],
        )
        .mount("/", routes![spa_handler])
        .register(
            "/",
            catchers![
                forbidden,
                too_many_requests,
                bad_request,
                internal_error,
                not_found
            ],
        );

    Ok(rocket.into())
}