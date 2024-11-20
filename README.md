# STAR Voting Platform (Alpha v0.1)

A full-stack Rust implementation of STAR (Score Then Automatic Runoff) voting system. STAR voting is an electoral system that combines the expressiveness of score voting with an automatic runoff between the two highest-rated candidates to ensure majority support.

> **Note**: This is the base version (Alpha v0.1) of the STAR Voting Platform.

## Features

- Score-based ballot casting (0-5 stars)
- Automatic runoff calculation
- Real-time vote tracking and results
- Detailed statistical analysis
- Head-to-head comparisons
- Comprehensive tie-breaking rules

## Architecture

- **Frontend**: Rust-based web application using Yew framework
- **Backend**: Rust server using Rocket with PostgreSQL database
- **Security**: 
  - CSRF protection
  - Rate limiting
  - hCaptcha verification
  - Browser fingerprinting
  - Input sanitization

## Project Structure

```
├── backend/      # Rocket web server
├── frontend/     # Yew web application
└── shared/       # Common types and logic
```

## Credits

Original STAR Voting system created by Mark Frohnmayer and Equal Vote Coalition.
Implementation by James Masland.

## License

MIT License