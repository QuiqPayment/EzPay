# EzPay Backend

Rust-based REST API for the EzPay payment infrastructure.

## Tech Stack

- **Rust** 1.70+
- **Axum** - Web framework
- **SQLx** - Database toolkit
- **PostgreSQL** - Database
- **Tokio** - Async runtime
- **Tower HTTP** - Middleware (CORS, tracing)

## Project Structure

```
backend/
├── src/
│   ├── main.rs          # Application entry point
│   ├── config.rs        # Environment configuration
│   ├── db.rs            # Database connection pool
│   ├── middleware.rs    # Error handling, auth, rate limiting
│   ├── models/          # Data models
│   │   ├── merchant.rs
│   │   └── payment.rs
│   └── routes/          # API endpoints
│       ├── health.rs
│       ├── merchants.rs
│       └── payments.rs
├── migrations/          # Database migrations
│   └── 001_initial.up.sql
├── Cargo.toml
└── .env.example
```

## Getting Started

### Prerequisites

- Rust 1.70+
- PostgreSQL 14+

### Setup

1. Copy environment variables:
```bash
cp .env.example .env
```

2. Configure `.env`:
```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/ezpay
PORT=3001
STELLAR_NETWORK=testnet
RUST_LOG=info
```

3. Create database:
```bash
createdb ezpay
```

4. Run migrations:
```bash
psql $DATABASE_URL -f migrations/001_initial.up.sql
```

5. Run the server:
```bash
cargo run
```

The API will be available at `http://localhost:3001`.

## API Endpoints

### Health Check
- `GET /api/health` - Health status

### Merchants
- `POST /api/merchants` - Create merchant
- `GET /api/merchants/:id` - Get merchant
- `PUT /api/merchants/:id` - Update merchant
- `DELETE /api/merchants/:id` - Deactivate merchant

### Payments
- `POST /api/payments` - Create payment
- `GET /api/payments/:id` - Get payment
- `GET /api/payments` - List payments

### Payment Requests
- `POST /api/payment-requests` - Create payment request
- `GET /api/payment-requests/:id` - Get payment request
- `DELETE /api/payment-requests/:id` - Cancel payment request

## Development

### Build
```bash
cargo build
```

### Run Tests
```bash
cargo test
```

### Run Linter
```bash
cargo clippy
```

### Format Code
```bash
cargo fmt
```

## Docker

Build and run with Docker:
```bash
docker build -t ezpay-backend .
docker run -p 3001:3001 --env-file .env ezpay-backend
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | - |
| `PORT` | Server port | 3001 |
| `STELLAR_NETWORK` | Stellar network (testnet/mainnet) | testnet |
| `RUST_LOG` | Log level | info |
| `DATABASE_MAX_CONNECTIONS` | Max DB connections | 10 |

Rate limits use an in-memory token bucket and are shared only within one server process. This is suitable for a single instance; multi-instance deployments need shared storage such as Redis. Each group supports a per-minute refill rate and bucket burst capacity:

| Group | Requests per minute | Environment variables |
|-------|---------------------|-----------------------|
| Public | 100 | `RATE_LIMIT_PUBLIC_PER_MINUTE`, `RATE_LIMIT_PUBLIC_BURST` |
| Authenticated | 1000 | `RATE_LIMIT_AUTHENTICATED_PER_MINUTE`, `RATE_LIMIT_AUTHENTICATED_BURST` |
| Payment | 10 | `RATE_LIMIT_PAYMENT_PER_MINUTE`, `RATE_LIMIT_PAYMENT_BURST` |
| Merchant | 100 | `RATE_LIMIT_MERCHANT_PER_MINUTE`, `RATE_LIMIT_MERCHANT_BURST` |
| Auth | 100 | `RATE_LIMIT_AUTH_PER_MINUTE`, `RATE_LIMIT_AUTH_BURST` |
| Health | 1000000 | `RATE_LIMIT_HEALTH_PER_MINUTE`, `RATE_LIMIT_HEALTH_BURST` |

The rate limiter keys authenticated callers by `RateLimitIdentity`, which must be inserted into request extensions by trusted authentication middleware after validating the caller's credentials. Requests without that identity are keyed by the remote IP address.

## Architecture

For detailed architecture information, see [ARCHITECTURE.md](../ARCHITECTURE.md).

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.
