# GraphQL Style Guide

A comprehensive guide for designing, implementing, and maintaining GraphQL APIs following best practices (2025/2026).

## Table of Contents

- [Schema Design](#schema-design)
- [Naming Conventions](#naming-conventions)
- [Type System Best Practices](#type-system-best-practices)
- [Resolver Patterns](#resolver-patterns)
- [Error Handling](#error-handling)
- [Performance Optimization](#performance-optimization)
- [Security Best Practices](#security-best-practices)
- [Testing](#testing)
- [Documentation](#documentation)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Schema Design

### Schema Organization

```graphql
# Good: Organize schema by domain
# types/user.graphql
type User {
  id: ID!
  email: String!
  name: String!
  posts(first: Int = 10, after: String): PostConnection!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type UserConnection {
  edges: [UserEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type UserEdge {
  node: User!
  cursor: String!
}

# types/post.graphql
type Post {
  id: ID!
  title: String!
  content: String!
  author: User!
  comments(first: Int = 10): CommentConnection!
  createdAt: DateTime!
  updatedAt: DateTime!
}

type PostConnection {
  edges: [PostEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type PostEdge {
  node: Post!
  cursor: String!
}

# types/common.graphql
type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

scalar DateTime

# Query and Mutation definitions
schema {
  query: Query
  mutation: Mutation
  subscription: Subscription
}

type Query {
  me: User
  user(id: ID!): User
  users(first: Int = 10, after: String): UserConnection!
  post(id: ID!): Post
  posts(first: Int = 10, after: String): PostConnection!
}

type Mutation {
  createPost(input: CreatePostInput!): CreatePostPayload!
  updatePost(id: ID!, input: UpdatePostInput!): UpdatePostPayload!
  deletePost(id: ID!): DeletePostPayload!
}

type Subscription {
  postCreated: Post!
  postUpdated: Post!
}
```

### Input Types

```graphql
# Good: Use input types for mutations
input CreatePostInput {
  title: String!
  content: String!
  tags: [String!]
  publishedAt: DateTime
}

input UpdatePostInput {
  title: String
  content: String
  tags: [String!]
  publishedAt: DateTime
}

input UserFilterInput {
  search: String
  minAge: Int
  maxAge: Int
  hasPosts: Boolean
}

input PostOrderInput {
  field: PostOrderField!
  direction: OrderDirection!
}

enum PostOrderField {
  CREATED_AT
  UPDATED_AT
  TITLE
}

enum OrderDirection {
  ASC
  DESC
}
```

### Payload Types

```graphql
# Good: Use payload types for mutations
type CreatePostPayload {
  post: Post
  errors: [Error!]!
  success: Boolean!
}

type UpdatePostPayload {
  post: Post
  errors: [Error!]!
  success: Boolean!
}

type DeletePostPayload {
  deletedId: ID
  errors: [Error!]!
  success: Boolean!
}

interface Error {
  message: String!
}

type ValidationError implements Error {
  message: String!
  field: String!
}

type AuthenticationError implements Error {
  message: String!
}

type NotFoundError implements Error {
  message: String!
  resource: String!
}
```

---

## Naming Conventions

### Type Naming

```graphql
# Good: PascalCase for types
type UserProfile { }
type BlogPost { }
type CommentThread { }

# Good: Descriptive type names
type User { }  # Good
type U { }     # Bad

# Good: Use Payload suffix for mutation results
type CreateUserPayload { }
type UpdatePostPayload { }

# Good: Use Input suffix for input types
input CreateUserInput { }
input UpdatePostInput { }

# Good: Use Connection suffix for pagination
type UserConnection { }
type PostConnection { }

# Good: Use Edge suffix for connection edges
type UserEdge { }
type PostEdge { }
```

### Field Naming

```graphql
# Good: camelCase for fields
type User {
  firstName: String!
  lastName: String!
  emailAddress: String!
  profilePictureUrl: String!
}

# Good: Boolean fields with is/has prefix
type User {
  isActive: Boolean!
  hasVerifiedEmail: Boolean!
  isAdmin: Boolean!
}

# Good: ID fields end with Id
type Comment {
  authorId: ID!
  postId: ID!
  parentId: ID
}
```

### Query/Mutation Naming

```graphql
# Good: Use verb + noun pattern
type Query {
  getUser(id: ID!): User
  listUsers(first: Int = 10): UserConnection!
  searchUsers(query: String!): [User!]!
}

type Mutation {
  createUser(input: CreateUserInput!): CreateUserPayload!
  updateUser(id: ID!, input: UpdatePostInput!): UpdatePostPayload!
  deleteUser(id: ID!): DeletePostPayload!
}

# Good: Subscription names are events
type Subscription {
  userCreated: User!
  userUpdated: User!
  userDeleted: ID!
}
```

---

## Type System Best Practices

### Nullability

```graphql
# Good: Use non-null fields when appropriate
type User {
  id: ID!           # Required - always present
  email: String!    # Required - always present
  name: String!     # Required - always present
  bio: String       # Optional - may be null
  website: String   # Optional - may be null
}

# Good: Required arguments
type Query {
  user(id: ID!): User          # id is required
  posts(first: Int = 10): [Post!]!  # first has default, returns non-null list of non-null posts
}

# Good: Nullable in lists vs list of nullables
posts: [Post!]!     # List itself is not null, contains non-null posts
posts: [Post]!      # List itself is not null, may contain null posts
posts: [Post!]      # List may be null, contains non-null posts
posts: [Post]       # Both list and items may be null
```

### Enums

```graphql
# Good: Use enums for fixed sets of values
enum UserRole {
  ADMIN
  MODERATOR
  USER
  GUEST
}

enum PostStatus {
  DRAFT
  PUBLISHED
  ARCHIVED
}

enum OrderDirection {
  ASC
  DESC
}

# Good: Use UPPER_CASE for enum values
enum NotificationType {
  EMAIL
  SMS
  PUSH
  IN_APP
}
```

### Custom Scalars

```graphql
# Good: Use custom scalars for specific data types
scalar DateTime
scalar Date
scalar Time
scalar JSON
scalar Upload
scalar URL

# With directives
scalar DateTime @specifiedBy(url: "https://scalars.graphql.org/DateTime")

# In resolver code
import { GraphQLScalarType, Kind } from 'graphql';

export const DateTimeScalar = new GraphQLScalarType({
  name: 'DateTime',
  description: 'DateTime custom scalar type',
  serialize(value: any) {
    return new Date(value).toISOString();
  },
  parseValue(value: any) {
    return new Date(value);
  },
  parseLiteral(ast) {
    if (ast.kind === Kind.STRING) {
      return new Date(ast.value);
    }
    return null;
  },
});
```

---

## Resolver Patterns

### Basic Resolver

```typescript
// Good: Resolver structure
// resolvers/user.resolver.ts
export const userResolvers = {
  Query: {
    user: async (_: any, { id }: { id: string }, { dataSources }: any) => {
      return dataSources.userAPI.getUserById(id);
    },

    users: async (_: any, { first = 10, after }: any, { dataSources }: any) => {
      return dataSources.userAPI.getUsers(first, after);
    },
  },

  User: {
    posts: async (user: User, { first = 10 }: any, { dataSources }: any) => {
      return dataSources.postAPI.getPostsByUserId(user.id, first);
    },

    email: async (user: User, _: any, { userId }: any) => {
      // Only return email if querying own profile
      if (user.id === userId) {
        return user.email;
      }
      return null;
    },
  },

  Mutation: {
    createUser: async (_: any, { input }: any, { dataSources }: any) => {
      const user = await dataSources.userAPI.createUser(input);
      return {
        code: 200,
        success: true,
        message: 'User created successfully',
        user,
      };
    },
  },
};
```

### DataLoader Pattern

```typescript
// Good: Use DataLoader to prevent N+1 queries
import DataLoader from 'dataloader';

const userLoader = new DataLoader(async (userIds: readonly string[]) => {
  const users = await db.query(
    'SELECT * FROM users WHERE id = ANY($1)',
    [userIds]
  );

  const userMap = new Map(users.map((user: User) => [user.id, user]));

  return userIds.map((id) => userMap.get(id) || null);
});

// In resolver
export const postResolvers = {
  Post: {
    author: async (post: Post, _: any, { userLoader }: any) => {
      return userLoader.load(post.authorId);
    },
  },
};
```

### Authentication in Resolvers

```typescript
// Good: Authentication middleware
export const authMiddleware = (resolver: any) => {
  return async (_: any, __: any, { user }: any) => {
    if (!user) {
      throw new AuthenticationError('You must be logged in');
    }
    return resolver(_, __, { user });
  };
};

// Usage
export const userResolvers = {
  Mutation: {
    updateProfile: authMiddleware(
      async (_: any, { input }: any, { user, dataSources }: any) => {
        return dataSources.userAPI.updateProfile(user.id, input);
      }
    ),
  },
};
```

---

## Error Handling

### Error Types

```typescript
// Good: Custom error classes
export class AuthenticationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'AuthenticationError';
    this.extensions = { code: 'AUTHENTICATION_ERROR' };
  }
}

export class ForbiddenError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ForbiddenError';
    this.extensions = { code: 'FORBIDDEN' };
  }
}

export class NotFoundError extends Error {
  constructor(resource: string, id: string) {
    super(`${resource} not found: ${id}`);
    this.name = 'NotFoundError';
    this.extensions = {
      code: 'NOT_FOUND',
      resource,
      id,
    };
  }
}

export class ValidationError extends Error {
  constructor(errors: any[]) {
    super('Validation failed');
    this.name = 'ValidationError';
    this.extensions = {
      code: 'VALIDATION_ERROR',
      errors,
    };
  }
}
```

### Error Handling in Resolvers

```typescript
// Good: Try-catch with specific errors
export const mutationResolvers = {
  Mutation: {
    createPost: async (_: any, { input }: any, { user, dataSources }: any) => {
      try {
        if (!user) {
          throw new AuthenticationError('You must be logged in');
        }

        const validation = validatePostInput(input);
        if (!validation.valid) {
          throw new ValidationError(validation.errors);
        }

        const post = await dataSources.postAPI.createPost(user.id, input);

        return {
          success: true,
          post,
          errors: [],
        };
      } catch (error) {
        if (error instanceof ValidationError) {
          return {
            success: false,
            post: null,
            errors: error.extensions.errors,
          };
        }
        throw error;
      }
    },
  },
};

// Good: Global error handler
export const errorFormatter = (error: any) => {
  return {
    message: error.message,
    code: error.extensions?.code || 'INTERNAL_SERVER_ERROR',
    ...(error.extensions && {
      ...error.extensions,
    }),
  };
};
```

---

## Performance Optimization

### Query Complexity Analysis

```typescript
// Good: Implement query complexity analysis
import { createComplexityLimitRule } from 'graphql-validation-complexity';

const complexityLimitRule = createComplexityLimitRule(100, {
  onCost: (cost: number) => {
    console.log(`Query cost: ${cost}`);
  },
  createError: (cost: number, max: number) => {
    return new Error(`Query complexity limit exceeded: ${cost} > ${max}`);
  },
});

// Usage in Apollo Server
const server = new ApolloServer({
  typeDefs,
  resolvers,
  validationRules: [complexityLimitRule],
});
```

### Query Depth Limiting

```typescript
// Good: Limit query depth
import { depthLimit } from 'graphql-depth-limit';

const server = new ApolloServer({
  typeDefs,
  resolvers,
  validationRules: [depthLimit(7)],
});
```

### Persisted Queries

```typescript
// Good: Use persisted queries for production
import { APQ } from 'graphql-apq-plugin';

const server = new ApolloServer({
  typeDefs,
  resolvers,
  plugins: [APQ()],
});
```

---

## Security Best Practices

### Query Whitelisting

```typescript
// Good: Persisted queries with whitelisting
import { use persistedQueries } from '@envelop/persisted-queries';

const server = new ApolloServer({
  typeDefs,
  resolvers,
  plugins: [
    use persistedQueries({
      ttl: 900, // 15 minutes
    }),
  ],
});
```

### Rate Limiting

```typescript
// Good: Rate limiting per user
import rateLimit from 'express-rate-limit';
import { RedisStore } from 'rate-limit-redis';

export const rateLimiter = rateLimit({
  store: new RedisStore({
    client: redis,
    prefix: 'rate-limit:',
  }),
  windowMs: 60 * 1000, // 1 minute
  max: 100, // 100 requests per minute
  keyGenerator: (req) => {
    return req.user?.id || req.ip;
  },
});

app.use('/graphql', rateLimiter);
```

---

## Testing

### Resolver Testing

```typescript
// Good: Unit test resolvers
import { resolvers } from '../resolvers/user.resolver';

describe('User Resolvers', () => {
  describe('Query.user', () => {
    it('should return user by ID', async () => {
      const mockUser = { id: '1', name: 'John', email: 'john@example.com' };
      const mockDataSources = {
        userAPI: {
          getUserById: jest.fn().mockResolvedValue(mockUser),
        },
      };

      const result = await resolvers.Query.user(
        null,
        { id: '1' },
        { dataSources: mockDataSources }
      );

      expect(result).toEqual(mockUser);
      expect(mockDataSources.userAPI.getUserById).toHaveBeenCalledWith('1');
    });

    it('should return null for non-existent user', async () => {
      const mockDataSources = {
        userAPI: {
          getUserById: jest.fn().mockResolvedValue(null),
        },
      };

      const result = await resolvers.Query.user(
        null,
        { id: '999' },
        { dataSources: mockDataSources }
      );

      expect(result).toBeNull();
    });
  });
});
```

---

## Documentation

### Schema Documentation

```graphql
# Good: Document types and fields
"""
Represents a user in the system.
Users can create posts and comments.
"""
type User {
  """
  The unique identifier of the user.
  This is generated automatically upon creation.
  """
  id: ID!

  """
  The email address of the user.
  Used for authentication and notifications.
  """
  email: String!

  """
  The display name of the user.
  Must be between 2 and 50 characters.
  """
  name: String!

  """
  The user's short biography.
  Maximum 500 characters.
  """
  bio: String

  """
  List of posts created by the user.
  Supports pagination with cursor-based connections.
  """
  posts(
    """
    Number of posts to return.
    Defaults to 10, maximum of 50.
    """
    first: Int = 10

    """
    Cursor for pagination.
    Use the endCursor from the previous page.
    """
    after: String
  ): PostConnection!

  """
  Timestamp when the user account was created.
  """
  createdAt: DateTime!

  """
  Timestamp when the user profile was last updated.
  """
  updatedAt: DateTime!
}
```

---

## Common Patterns

### Relay-Style Pagination

```graphql
type PostConnection {
  edges: [PostEdge!]!
  pageInfo: PageInfo!
  totalCount: Int!
}

type PostEdge {
  node: Post!
  cursor: String!
}

type PageInfo {
  hasNextPage: Boolean!
  hasPreviousPage: Boolean!
  startCursor: String
  endCursor: String
}

type Query {
  posts(
    """Returns the first n items"""
    first: Int = 10

    """Cursor for forward pagination"""
    after: String
  ): PostConnection!
}
```

---

## Anti-Patterns to Avoid

### Don't Over-Nest

```graphql
# Bad: Excessive nesting
type Query {
  user(id: ID!): User
}

type User {
  posts: [Post!]!
}

type Post {
  comments: [Comment!]!
}

type Comment {
  author: User!
}

# Query becomes deeply nested
query {
  user(id: "1") {
    posts {
      comments {
        author {
          posts {
            comments {
              author {
                # And so on...
              }
            }
          }
        }
      }
    }
  }
}

# Good: Flatten where possible
type Query {
  user(id: ID!): User
  posts(userId: ID!): [Post!]!
  comments(postId: ID!): [Comment!]!
}
```

---

## Additional Resources

- [GraphQL Specification](https://spec.graphql.org/)
- [Apollo Best Practices](https://www.apollographql.com/docs/technical-best-practices/)
- [GraphQL Foundation](https://graphql.org/)
- [Relay Specification](https://relay.dev/docs/guides/graphql-server-specification/)
