# Docker Style Guide

A comprehensive guide for creating efficient, secure, and maintainable Dockerfiles and Docker Compose configurations (2025/2026).

## Table of Contents

- [Dockerfile Best Practices](#dockerfile-best-practices)
- [Multi-Stage Builds](#multi-stage-builds)
- [Image Optimization](#image-optimization)
- [Security Practices](#security-practices)
- [Docker Compose](#docker-compose)
- [Container Orchestration](#container-orchestration)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Dockerfile Best Practices

### Basic Structure

```dockerfile
# Good: Use specific base image version
FROM node:20-alpine AS base

# Good: Set working directory
WORKDIR /app

# Good: Use non-root user
FROM base AS deps
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001

# Good: Copy package files first for better caching
COPY package.json package-lock.json ./

# Good: Install dependencies
RUN npm ci --only=production && \
    npm cache clean --force

# Good: Copy application files
FROM base AS builder
COPY --from=deps /app/node_modules ./node_modules
COPY . .

# Good: Build application
RUN npm run build

# Good: Production stage
FROM base AS runner
WORKDIR /app

ENV NODE_ENV=production

# Good: Create non-root user
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nextjs -u 1001

# Good: Copy necessary files only
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static

USER nextjs

EXPOSE 3000

ENV PORT 3000
ENV HOSTNAME "0.0.0.0"

# Good: Use health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node healthcheck.js || exit 1

CMD ["node", "server.js"]
```

### Language-Specific Patterns

#### Node.js

```dockerfile
# Multi-stage Node.js application
FROM node:20-alpine AS base
WORKDIR /app

# Dependencies stage
FROM base AS deps
COPY package.json package-lock.json ./
RUN npm ci

# Builder stage
FROM base AS builder
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

# Production stage
FROM node:20-alpine AS runner
WORKDIR /app

ENV NODE_ENV=production

RUN addgroup -g 1001 -S nodejs && \
    adduser -S nodejs -u 1001

COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY package.json ./

USER nodejs

EXPOSE 3000

CMD ["node", "dist/index.js"]
```

#### Python

```dockerfile
# Multi-stage Python application
FROM python:3.12-slim AS base

WORKDIR /app

# Install system dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        gcc \
        && rm -rf /var/lib/apt/lists/*

# Dependencies stage
FROM base AS deps
COPY requirements.txt .
RUN pip install --user --no-cache-dir -r requirements.txt

# Builder stage
FROM base AS builder
COPY --from=deps /root/.local /root/.local
COPY . .
RUN pip install --user --no-cache-dir -e .

# Production stage
FROM python:3.12-slim AS runner

WORKDIR /app

ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1

RUN addgroup -g 1001 -S python && \
    adduser -S python -u 1001

COPY --from=builder /root/.local /root/.local
COPY --from=builder --chown=python:python /app /app

USER python

EXPOSE 8000

CMD ["python", "-m", "uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8000"]
```

#### Rust

```dockerfile
# Multi-stage Rust application
FROM rust:1.75-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source and build
COPY src ./src
RUN cargo build --release && \
    strip target/release/myapp

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache ca-certificates

WORKDIR /app

COPY --from=builder /app/target/release/myapp /app/myapp

EXPOSE 8080

CMD ["/app/myapp"]
```

---

## Multi-Stage Builds

```dockerfile
# Good: Use multi-stage builds to reduce image size
FROM golang:1.21-alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache git make

# Copy go mod files for caching
COPY go.* ./
RUN go mod download

# Copy source and build
COPY . .
RUN CGO_ENABLED=0 go build -ldflags="-w -s" -o /app/main .

# Minimal runtime image
FROM alpine:3.19

RUN apk add --no-cache ca-certificates tzdata

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/main /app/main

# Use non-root user
RUN addgroup -g 1001 -S appuser && \
    adduser -S appuser -u 1001
USER appuser

EXPOSE 8080

CMD ["/app/main"]
```

---

## Image Optimization

### Layer Caching

```dockerfile
# Good: Order instructions to maximize layer caching
FROM node:20-alpine

WORKDIR /app

# Copy dependency files first (change less often)
COPY package.json package-lock.json ./

# Install dependencies (cached unless package files change)
RUN npm ci --only=production

# Copy application files (change more often)
COPY . .

# Build (cached unless source changes)
RUN npm run build

# Bad: All files copied at once (no caching benefit)
FROM node:20-alpine

WORKDIR /app

COPY . .
RUN npm install
RUN npm run build
```

### Minimizing Image Size

```dockerfile
# Good: Use alpine-based images
FROM node:20-alpine  # ~120MB
# VS
FROM node:20        # ~900MB

# Good: Use .dockerignore
# .dockerignore file
node_modules
npm-debug.log
.git
.env
.env.local
coverage
.vscode
*.md

# Good: Clean up in same layer
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Good: Multi-stage for final image
FROM node:20-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Final image only contains production files
FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
CMD ["node", "dist/index.js"]
```

---

## Security Practices

### Scanning for Vulnerabilities

```bash
# Good: Scan images with Trivy
trivy image myapp:latest

# Good: Use Docker Scout
docker scout quickview myapp:latest
docker scout cves myapp:latest

# Good: Scan during build
FROM node:20-alpine
RUN apk add --no-cache trivy
RUN trivy filesystem --skip-db-update --no-progress /
```

### Running as Non-Root

```dockerfile
# Good: Create and use non-root user
FROM node:20-alpine

# Create user and group
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nodejs -u 1001

WORKDIR /app

# Copy and install as root
COPY package*.json ./
RUN npm ci --only=production

# Copy app files
COPY . .

# Change ownership
RUN chown -R nodejs:nodejs /app

# Switch to non-root user
USER nodejs

EXPOSE 3000

CMD ["node", "index.js"]
```

### Minimizing Attack Surface

```dockerfile
# Good: Minimal base image
FROM alpine:3.19

# Only install necessary packages
RUN apk add --no-cache \
    ca-certificates \
    tzdata

# No build tools, compilers, or debug tools

WORKDIR /app
COPY --from=builder /app/myapp .
CMD ["./myapp"]
```

---

## Docker Compose

### Development Environment

```yaml
# Good: Docker Compose for development
version: '3.9'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
      target: development
    volumes:
      - .:/app
      - /app/node_modules
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=development
      - DATABASE_URL=postgres://postgres:password@db:5432/myapp
    depends_on:
      - db
      - redis

  db:
    image: postgres:16-alpine
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      - POSTGRES_DB=myapp
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    ports:
      - "6379:6379"

volumes:
  postgres_data:
  redis_data:
```

### Production Environment

```yaml
# Good: Production-ready Docker Compose
version: '3.9'

services:
  app:
    image: myapp:latest
    restart: always
    environment:
      - NODE_ENV=production
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
    depends_on:
      - db
      - redis
    networks:
      - app_network
    deploy:
      replicas: 3
      resources:
        limits:
          cpus: '0.5'
          memory: 512M
        reservations:
          cpus: '0.25'
          memory: 256M
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  db:
    image: postgres:16-alpine
    restart: always
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      - POSTGRES_DB=${POSTGRES_DB}
      - POSTGRES_USER=${POSTGRES_USER}
      - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
    networks:
      - app_network
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER}"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    restart: always
    volumes:
      - redis_data:/data
    networks:
      - app_network
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 3s
      retries: 3

  nginx:
    image: nginx:alpine
    restart: always
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - app
    networks:
      - app_network

networks:
  app_network:
    driver: bridge

volumes:
  postgres_data:
  redis_data:
```

---

## Container Orchestration

### Kubernetes Deployment

```yaml
# Good: Kubernetes deployment manifest
apiVersion: apps/v1
kind: Deployment
metadata:
  name: myapp
  labels:
    app: myapp
spec:
  replicas: 3
  selector:
    matchLabels:
      app: myapp
  template:
    metadata:
      labels:
        app: myapp
    spec:
      containers:
      - name: myapp
        image: myapp:latest
        ports:
        - containerPort: 3000
        env:
        - name: NODE_ENV
          value: "production"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: myapp-secrets
              key: database-url
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi"
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: myapp-service
spec:
  selector:
    app: myapp
  ports:
  - protocol: TCP
    port: 80
    targetPort: 3000
  type: LoadBalancer
```

---

## Common Patterns

### Health Checks

```dockerfile
# Good: Implement health checks
FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

# Create health check script
RUN echo 'const http = require("http");\
  const options = { host: "localhost", port: 3000, path: "/health", timeout: 2000 };\
  const request = http.request(options, (res) => {\
    console.log(`Health check: ${res.statusCode}`);\
    process.exit(res.statusCode === 200 ? 0 : 1);\
  });\
  request.on("error", () => process.exit(1));\
  request.end();' > /healthcheck.js

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD node /healthcheck.js || exit 1

EXPOSE 3000

CMD ["node", "index.js"]
```

---

## Anti-Patterns to Avoid

### Don't Run as Root

```dockerfile
# Bad: Running as root
FROM node:20-alpine
WORKDIR /app
COPY . .
RUN npm install
USER root
CMD ["node", "index.js"]

# Good: Non-root user
FROM node:20-alpine
RUN addgroup -g 1001 -S nodejs && \
    adduser -S nodejs -u 1001
WORKDIR /app
COPY --chown=nodejs:nodejs . .
RUN npm ci --only=production
USER nodejs
CMD ["node", "index.js"]
```

### Don't Cache Credentials

```dockerfile
# Bad: Credentials in image
FROM node:20-alpine
ENV API_KEY=secret-key-123
COPY . .

# Good: Use runtime secrets or mounted files
FROM node:20-alpine
COPY . .
# API_KEY passed at runtime
# docker run -e API_KEY=secret-key-123 myapp
```

---

## Additional Resources

- [Dockerfile Best Practices](https://docs.docker.com/develop/develop-images/dockerfile_best-practices/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Docker Security](https://docs.docker.com/engine/security/)
