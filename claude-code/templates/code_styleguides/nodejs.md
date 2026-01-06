# Node.js Style Guide

A comprehensive guide for building scalable, maintainable Node.js applications following best practices from the Node.js community (2025/2026).

## Table of Contents

- [Project Structure](#project-structure)
- [Error Handling](#error-handling)
- [Async Patterns](#async-patterns)
- [Module Organization](#module-organization)
- [API Design](#api-design)
- [Security Best Practices](#security-best-practices)
- [Performance Optimization](#performance-optimization)
- [Testing](#testing)
- [Logging and Monitoring](#logging-and-monitoring)
- [Tooling and Configuration](#tooling-and-configuration)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Project Structure

### Recommended Structure

```bash
# Good: Layered architecture with feature-based organization
src/
├── config/
│   ├── database.ts
│   ├── redis.ts
│   └── env.ts
├── controllers/
│   ├── user.controller.ts
│   └── auth.controller.ts
├── services/
│   ├── user.service.ts
│   └── auth.service.ts
├── repositories/
│   ├── user.repository.ts
│   └── auth.repository.ts
├── middleware/
│   ├── auth.middleware.ts
│   ├── error.middleware.ts
│   └── validation.middleware.ts
├── models/
│   ├── user.model.ts
│   └── types.ts
├── routes/
│   ├── user.routes.ts
│   └── auth.routes.ts
├── utils/
│   ├── logger.ts
│   ├── validator.ts
│   └── helpers.ts
├── constants/
│   └── errors.ts
├── tests/
│   ├── unit/
│   └── integration/
├── app.ts
└── server.ts
```

### Entry Points

```typescript
// Good: Separate app and server setup
// src/app.ts - Application configuration
import express from 'express';
import { userRoutes } from './routes/user.routes';
import { authRoutes } from './routes/auth.routes';
import { errorHandler } from './middleware/error.middleware';
import { logger } from './utils/logger';

export function createApp(): express.Application {
  const app = express();

  // Middleware
  app.use(express.json());
  app.use(express.urlencoded({ extended: true }));
  app.use(logger);

  // Routes
  app.use('/api/users', userRoutes);
  app.use('/api/auth', authRoutes);

  // Error handling
  app.use(errorHandler);

  return app;
}

// src/server.ts - Server startup
import { createApp } from './app';
import { config } from './config/env';

const app = createApp();

app.listen(config.port, () => {
  console.log(`Server running on port ${config.port}`);
});
```

---

## Error Handling

### Custom Error Classes

```typescript
// Good: Create custom error types
// src/utils/errors.ts
export class AppError extends Error {
  constructor(
    public message: string,
    public statusCode: number = 500,
    public isOperational: boolean = true,
    public code?: string
  ) {
    super(message);
    this.name = this.constructor.name;
    Error.captureStackTrace(this, this.constructor);
  }
}

export class ValidationError extends AppError {
  constructor(message: string, public field?: string) {
    super(message, 400, true, 'VALIDATION_ERROR');
  }
}

export class NotFoundError extends AppError {
  constructor(resource: string, id: string) {
    super(`${resource} not found: ${id}`, 404, true, 'NOT_FOUND');
  }
}

export class UnauthorizedError extends AppError {
  constructor(message: string = 'Unauthorized') {
    super(message, 401, true, 'UNAUTHORIZED');
  }
}

export class ForbiddenError extends AppError {
  constructor(message: string = 'Forbidden') {
    super(message, 403, true, 'FORBIDDEN');
  }
}
```

### Error Handling Middleware

```typescript
// Good: Centralized error handling
// src/middleware/error.middleware.ts
import { Request, Response, NextFunction } from 'express';
import { logger } from '../utils/logger';
import { AppError } from '../utils/errors';

export function errorHandler(
  err: Error,
  req: Request,
  res: Response,
  next: NextFunction
): void {
  if (err instanceof AppError) {
    logger.warn({
      message: err.message,
      statusCode: err.statusCode,
      code: err.code,
      url: req.url,
      method: req.method,
      ip: req.ip,
    });

    res.status(err.statusCode).json({
      status: 'error',
      message: err.message,
      code: err.code,
    });
    return;
  }

  // Unexpected errors
  logger.error({
    message: err.message,
    stack: err.stack,
    url: req.url,
    method: req.method,
    ip: req.ip,
  });

  const isDevelopment = process.env.NODE_ENV === 'development';

  res.status(500).json({
    status: 'error',
    message: isDevelopment ? err.message : 'Internal server error',
    ...(isDevelopment && { stack: err.stack }),
  });
}

// Good: Async error wrapper
export function asyncHandler(
  fn: (req: Request, res: Response, next: NextFunction) => Promise<any>
) {
  return (req: Request, res: Response, next: NextFunction) => {
    Promise.resolve(fn(req, res, next)).catch(next);
  };
}
```

### Error Handling in Controllers

```typescript
// Good: Controllers throw errors, middleware handles them
// src/controllers/user.controller.ts
import { asyncHandler } from '../middleware/error.middleware';
import { NotFoundError, ValidationError } from '../utils/errors';

export class UserController {
  getUser = asyncHandler(async (req, res) => {
    const { id } = req.params;

    const user = await userService.findById(id);
    if (!user) {
      throw new NotFoundError('User', id);
    }

    res.json(user);
  });

  createUser = asyncHandler(async (req, res) => {
    const { email, name } = req.body;

    if (!email || !name) {
      throw new ValidationError('Email and name are required');
    }

    const user = await userService.create({ email, name });
    res.status(201).json(user);
  });
}
```

### Global Error Handlers

```typescript
// Good: Handle uncaught exceptions and rejections
// src/server.ts
process.on('uncaughtException', (error: Error) => {
  logger.error('Uncaught Exception:', error);
  process.exit(1);
});

process.on('unhandledRejection', (reason: unknown, promise: Promise<any>) => {
  logger.error('Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

process.on('SIGTERM', () => {
  logger.info('SIGTERM received, shutting down gracefully');
  process.exit(0);
});

process.on('SIGINT', () => {
  logger.info('SIGINT received, shutting down gracefully');
  process.exit(0);
});
```

---

## Async Patterns

### Async/Await Best Practices

```typescript
// Good: Use async/await for better readability
async function getUserWithPosts(userId: string): Promise<UserWithPosts> {
  const user = await userRepository.findById(userId);
  if (!user) {
    throw new NotFoundError('User', userId);
  }

  const posts = await postRepository.findByUserId(userId);

  return {
    ...user,
    posts,
  };
}

// Good: Use Promise.all for parallel operations
async function getUserDashboard(userId: string): Promise<Dashboard> {
  const [user, posts, notifications] = await Promise.all([
    userRepository.findById(userId),
    postRepository.findByUserId(userId),
    notificationRepository.findUnread(userId),
  ]);

  return { user, posts, notifications };
}

// Good: Use Promise.allSettled for independent operations
async function sendNotifications(users: User[]): Promise<void> {
  const results = await Promise.allSettled(
    users.map(user => emailService.send(user.email))
  );

  const failed = results.filter(r => r.status === 'rejected');
  if (failed.length > 0) {
    logger.warn(`${failed.length} notifications failed to send`);
  }
}
```

### Avoiding Callback Hell

```typescript
// Bad: Callback hell
function getUserData(userId: string, callback: (err: Error | null, data?: any) => void) {
  userRepository.findById(userId, (err, user) => {
    if (err) return callback(err);
    postRepository.findByUserId(userId, (err, posts) => {
      if (err) return callback(err);
      callback(null, { user, posts });
    });
  });
}

// Good: Async/await
async function getUserData(userId: string): Promise<any> {
  const user = await userRepository.findById(userId);
  const posts = await postRepository.findByUserId(userId);
  return { user, posts };
}
```

---

## Module Organization

### Export Strategies

```typescript
// Good: Named exports for most modules
// src/utils/logger.ts
export function info(message: string): void {
  console.log(`[INFO] ${message}`);
}

export function error(message: string): void {
  console.error(`[ERROR] ${message}`);
}

export function warn(message: string): void {
  console.warn(`[WARN] ${message}`);
}

// Good: Default export for main class or function
// src/services/user.service.ts
export default class UserService {
  // Implementation
}

// Good: Barrel exports for clean imports
// src/models/index.ts
export * from './user.model';
export * from './post.model';
export * from './types';
```

### Import Ordering

```typescript
// Good: Organize imports by group
// 1. Node.js built-ins
import path from 'path';
import fs from 'fs';

// 2. External dependencies
import express from 'express';
import _ from 'lodash';
import { z } from 'zod';

// 3. Internal modules
import { config } from '../config';
import { logger } from '../utils/logger';

// 4. Types
import type { User, CreateUserDto } from '../types';

// 5. Relative imports
import { userRepository } from './user.repository';
```

---

## API Design

### RESTful Conventions

```typescript
// Good: RESTful route design
// src/routes/user.routes.ts
import { Router } from 'express';
import { UserController } from '../controllers/user.controller';
import { authenticate } from '../middleware/auth.middleware';

const router = Router();
const userController = new UserController();

// GET /api/users - List users
router.get('/', userController.listUsers);

// GET /api/users/:id - Get user by ID
router.get('/:id', userController.getUser);

// POST /api/users - Create user
router.post('/', userController.createUser);

// PUT /api/users/:id - Update user (full update)
router.put('/:id', userController.updateUser);

// PATCH /api/users/:id - Partial update
router.patch('/:id', userController.patchUser);

// DELETE /api/users/:id - Delete user
router.delete('/:id', userController.deleteUser);

// GET /api/users/:id/posts - Get user's posts (nested routes)
router.get('/:id/posts', userController.getUserPosts);

export { router as userRoutes };
```

### Request Validation

```typescript
// Good: Use validation middleware
// src/middleware/validation.middleware.ts
import { Request, Response, NextFunction } from 'express';
import { AnyZodObject, ZodError } from 'zod';

export function validate(schema: AnyZodObject) {
  return async (req: Request, res: Response, next: NextFunction) => {
    try {
      await schema.parseAsync({
        body: req.body,
        query: req.query,
        params: req.params,
      });
      next();
    } catch (error) {
      if (error instanceof ZodError) {
        res.status(400).json({
          status: 'error',
          errors: error.errors,
        });
      } else {
        next(error);
      }
    }
  };
}

// Usage
// src/routes/user.routes.ts
import { z } from 'zod';
import { validate } from '../middleware/validation.middleware';

const createUserSchema = z.object({
  body: z.object({
    name: z.string().min(2).max(100),
    email: z.string().email(),
    password: z.string().min(8),
  }),
});

router.post('/', validate(createUserSchema), userController.createUser);
```

---

## Security Best Practices

### Environment Variables

```typescript
// Good: Validate environment variables
// src/config/env.ts
import { z } from 'zod';

const envSchema = z.object({
  NODE_ENV: z.enum(['development', 'production', 'test']).default('development'),
  PORT: z.string().transform(Number).default('3000'),
  DATABASE_URL: z.string(),
  REDIS_URL: z.string().optional(),
  JWT_SECRET: z.string().min(32),
  CORS_ORIGIN: z.string().default('*'),
});

const env = envSchema.parse(process.env);

export { env as config };

// Good: Use config throughout app
import { config } from './config/env';
```

### Security Headers

```typescript
// Good: Set security headers
// src/middleware/security.middleware.ts
import { Request, Response, NextFunction } from 'express';
import helmet from 'helmet';
import cors from 'cors';

export function securityMiddleware(app: express.Application): void {
  // Security headers
  app.use(helmet({
    contentSecurityPolicy: {
      directives: {
        defaultSrc: ["'self'"],
        styleSrc: ["'self'", "'unsafe-inline'"],
        scriptSrc: ["'self'"],
        imgSrc: ["'self'", 'data:', 'https:'],
      },
    },
    hsts: {
      maxAge: 31536000,
      includeSubDomains: true,
      preload: true,
    },
  }));

  // CORS
  app.use(cors({
    origin: config.CORS_ORIGIN,
    credentials: true,
    methods: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'],
    allowedHeaders: ['Content-Type', 'Authorization'],
  }));

  // Rate limiting
  import rateLimit from 'express-rate-limit';
  app.use(rateLimit({
    windowMs: 15 * 60 * 1000, // 15 minutes
    max: 100, // limit each IP to 100 requests per windowMs
    message: 'Too many requests from this IP',
  }));
}
```

### Input Sanitization

```typescript
// Good: Sanitize user input
import { z } from 'zod';

const userInputSchema = z.object({
  name: z.string().min(2).max(100).trim(),
  email: z.string().email().toLowerCase(),
  bio: z.string().max(500).transform(val => val.replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')),
});
```

---

## Performance Optimization

### Caching Strategies

```typescript
// Good: In-memory caching with Node-cache
import NodeCache from 'node-cache';

const cache = new NodeCache({
  stdTTL: 600, // 10 minutes
  checkperiod: 620,
  useClones: false,
});

export async function getCachedUser(id: string): Promise<User | null> {
  const cached = cache.get<User>(`user:${id}`);
  if (cached) {
    return cached;
  }

  const user = await userRepository.findById(id);
  if (user) {
    cache.set(`user:${id}`, user);
  }
  return user;
}

// Good: Cache invalidation
export async function updateUser(id: string, data: UpdateUserDto): Promise<User> {
  const user = await userRepository.update(id, data);
  cache.del(`user:${id}`);
  return user;
}
```

### Database Optimization

```typescript
// Good: Use connection pooling
// src/config/database.ts
import { Pool } from 'pg';

const pool = new Pool({
  connectionString: config.DATABASE_URL,
  max: 20,
  idleTimeoutMillis: 30000,
  connectionTimeoutMillis: 2000,
});

export { pool };

// Good: Use prepared statements
async function getUserById(id: string): Promise<User | null> {
  const result = await pool.query(
    'SELECT * FROM users WHERE id = $1',
    [id]
  );
  return result.rows[0] || null;
}
```

---

## Testing

### Unit Testing

```typescript
// Good: Test services in isolation
// tests/services/user.service.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { UserService } from '../../src/services/user.service';
import { UserRepository } from '../../src/repositories/user.repository';

describe('UserService', () => {
  let userService: UserService;
  let mockUserRepository: UserRepository;

  beforeEach(() => {
    mockUserRepository = {
      findById: vi.fn(),
      create: vi.fn(),
      update: vi.fn(),
      delete: vi.fn(),
    } as any;
    userService = new UserService(mockUserRepository);
  });

  describe('getUserById', () => {
    it('should return a user if found', async () => {
      const mockUser = { id: '1', name: 'John', email: 'john@example.com' };
      vi.spyOn(mockUserRepository, 'findById').mockResolvedValue(mockUser);

      const result = await userService.getUserById('1');

      expect(result).toEqual(mockUser);
      expect(mockUserRepository.findById).toHaveBeenCalledWith('1');
    });

    it('should throw NotFoundError if user not found', async () => {
      vi.spyOn(mockUserRepository, 'findById').mockResolvedValue(null);

      await expect(userService.getUserById('1')).rejects.toThrow('User not found');
    });
  });
});
```

### Integration Testing

```typescript
// Good: Test API endpoints
// tests/integration/users.test.ts
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import request from 'supertest';
import { app } from '../../src/app';
import { pool } from '../../src/config/database';

describe('User API', () => {
  beforeAll(async () => {
    // Setup test database
  });

  afterAll(async () => {
    await pool.end();
  });

  describe('POST /api/users', () => {
    it('should create a new user', async () => {
      const response = await request(app)
        .post('/api/users')
        .send({
          name: 'John Doe',
          email: 'john@example.com',
          password: 'SecurePass123!',
        })
        .expect(201);

      expect(response.body).toHaveProperty('id');
      expect(response.body.name).toBe('John Doe');
      expect(response.body).not.toHaveProperty('password');
    });

    it('should return 400 for invalid input', async () => {
      await request(app)
        .post('/api/users')
        .send({
          name: 'J',
          email: 'invalid-email',
        })
        .expect(400);
    });
  });
});
```

---

## Logging and Monitoring

### Structured Logging

```typescript
// Good: Use structured logging
// src/utils/logger.ts
import pino from 'pino';

const logger = pino({
  level: process.env.LOG_LEVEL || 'info',
  formatters: {
    level: (label) => {
      return { level: label };
    },
  },
  timestamp: pino.stdTimeFunctions.isoTime,
});

export { logger };

// Usage
logger.info({
  message: 'User created',
  userId: user.id,
  email: user.email,
});

logger.error({
  message: 'Database error',
  error: error.message,
  stack: error.stack,
});
```

---

## Tooling and Configuration

### ESLint Configuration

```json
{
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended",
    "plugin:@typescript-eslint/recommended-requiring-type-checking",
    "prettier"
  ],
  "parser": "@typescript-eslint/parser",
  "parserOptions": {
    "ecmaVersion": 2022,
    "sourceType": "module",
    "project": "./tsconfig.json"
  },
  "rules": {
    "@typescript-eslint/no-unused-vars": "error",
    "@typescript-eslint/no-explicit-any": "warn",
    "@typescript-eslint/explicit-function-return-type": "warn",
    "no-console": "warn"
  }
}
```

---

## Common Patterns

### Dependency Injection

```typescript
// Good: Constructor injection for testability
export class UserService {
  constructor(
    private userRepository: UserRepository,
    private emailService: EmailService,
    private logger: Logger
  ) {}

  async createUser(data: CreateUserDto): Promise<User> {
    const user = await this.userRepository.create(data);
    await this.emailService.sendWelcomeEmail(user.email);
    this.logger.info(`User created: ${user.id}`);
    return user;
  }
}

// Usage
const userService = new UserService(
  new UserRepository(),
  new EmailService(),
  new ConsoleLogger()
);
```

---

## Anti-Patterns to Avoid

### Don't Mix Concerns

```typescript
// Bad: Controller doing database work
async function getUser(req: Request, res: Response) {
  const user = await db.query('SELECT * FROM users WHERE id = $1', [req.params.id]);
  res.json(user);
}

// Good: Separate layers
// Controller
async function getUser(req: Request, res: Response) {
  const user = await userService.getUserById(req.params.id);
  res.json(user);
}

// Service
async function getUserById(id: string): Promise<User> {
  return userRepository.findById(id);
}

// Repository
async function findById(id: string): Promise<User | null> {
  const result = await db.query('SELECT * FROM users WHERE id = $1', [id]);
  return result.rows[0];
}
```

---

## Additional Resources

- [Node.js Best Practices](https://github.com/goldbergyoni/nodebestpractices)
- [Node.js Documentation](https://nodejs.org/docs)
- [Express.js Guide](https://expressjs.com/en/guide/routing.html)
- [TypeScript Node Starter](https://github.com/microsoft/TypeScript-Node-Starter)
