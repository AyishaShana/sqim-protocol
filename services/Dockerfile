FROM golang:1.23-alpine AS build

WORKDIR /src
COPY go.mod ./
RUN go mod download
COPY . .
ARG SERVICE
RUN CGO_ENABLED=0 GOOS=linux go build -o /out/service ./${SERVICE}

FROM alpine:3.20
RUN adduser -D -H sqim
WORKDIR /app
COPY --from=build /out/service /app/service
COPY db/schema.sql /app/db/schema.sql
USER sqim
ENTRYPOINT ["/app/service"]
