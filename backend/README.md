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
psql $DATABASE_URL -f migrations/002_stellar_events.up.sql
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
| `STELLAR_RPC_URL` | Soroban RPC endpoint for contract events | https://soroban-testnet.stellar.org |
| `STELLAR_CONTRACT_ID` | EzPay Soroban contract address; enables event synchronization | - |
| `STELLAR_EVENT_START_LEDGER` | First ledger to scan when no cursor exists; defaults to current ledger | current ledger |
| `STELLAR_EVENT_POLL_INTERVAL_SECS` | Delay between Soroban RPC event polls | 3 |
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

### Contract event synchronization

Soroban contract events are fetched from Stellar RPC `getEvents`; Horizon's event
stream does not expose Soroban contract events. The listener filters by
`STELLAR_CONTRACT_ID`, stores a cursor and processed event IDs in PostgreSQL,
and applies each projection update in the same transaction as cursor advancement.
Set `STELLAR_EVENT_START_LEDGER` to the contract deployment ledger for an initial
backfill. Without it, a first run begins at the latest ledger. RPC event history
is retention-limited, so an outage longer than the endpoint's retention window
requires a rescan from an available ledger or a contract-state reconciliation.
Chain projections are kept in `chain_merchants` and `chain_payment_requests`;
they do not create login accounts in the separate `merchants` table.

The processor handles `mrch_reg` and `mrch_upd` with the emitted `MerchantData`,
and `mrch_off` with its merchant-address topic. `pay_req` carries a
`PaymentRequest`; `pay_done` carries request ID and payer topics plus the
`(net_amount, fee_amount, token)` value; `pay_cncl` carries request ID and
merchant address. Addresses are stored as base64-encoded ScVal XDR in the chain
projection tables, and request IDs as lowercase hex. Every event ID is recorded
in `stellar_contract_events`; the projection write and cursor update commit
together. Malformed events are recorded with `processing_error` and skipped so
one bad payload cannot block later ledger events.

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.
