FROM golang:1.25-bookworm AS build

WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY . .
ARG SERVICE
RUN CGO_ENABLED=0 GOOS=linux go build -o /out/service ./${SERVICE}

FROM debian:bookworm-slim
ARG STELLAR_CLI_VERSION=27.0.0
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && curl -fsSL "https://github.com/stellar/stellar-cli/releases/download/v${STELLAR_CLI_VERSION}/stellar-cli-${STELLAR_CLI_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
      | tar -xz -C /usr/local/bin \
    && stellar --version \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --no-create-home sqim
WORKDIR /app
COPY --from=build /out/service /app/service
COPY db/schema.sql /app/db/schema.sql
COPY --chown=sqim:sqim backtester/data /app/data/backtests
USER sqim
ENTRYPOINT ["/app/service"]
