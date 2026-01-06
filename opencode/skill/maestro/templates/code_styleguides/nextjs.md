# Next.js Style Guide

A comprehensive guide for building modern Next.js applications using the App Router, Server Components, and React Server Actions (2025/2026).

## Table of Contents

- [App Router Fundamentals](#app-router-fundamentals)
- [Server vs Client Components](#server-vs-client-components)
- [Data Fetching Patterns](#data-fetching-patterns)
- [Server Actions](#server-actions)
- [Routing and Navigation](#routing-and-navigation)
- [Performance Optimization](#performance-optimization)
- [API Routes](#api-routes)
- [Authentication](#authentication)
- [State Management](#state-management)
- [Testing Strategies](#testing-strategies)
- [Deployment Best Practices](#deployment-best-practices)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## App Router Fundamentals

### Project Structure

```typescript
// Good: Recommended Next.js 15+ App Router structure
app/
├── (auth)/
│   ├── login/
│   │   └── page.tsx
│   ├── register/
│   │   └── page.tsx
│   └── layout.tsx
├── (dashboard)/
│   ├── dashboard/
│   │   └── page.tsx
│   ├── settings/
│   │   └── page.tsx
│   └── layout.tsx
├── api/
│   ├── users/
│   │   └── route.ts
│   └── webhooks/
│       └── route.ts
├── layout.tsx
├── page.tsx
├── error.tsx
├── not-found.tsx
└── loading.tsx

components/
├── ui/
│   ├── button.tsx
│   ├── input.tsx
│   └── card.tsx
├── forms/
│   └── login-form.tsx
└── layouts/
    └── header.tsx

lib/
├── db.ts
├── auth.ts
├── utils.ts
└── validations.ts

public/
└── images/

styles/
└── globals.css
```

### Route Groups

```typescript
// Good: Use route groups for organization without affecting URL
// app/(auth)/login/page.tsx -> /login
// app/(dashboard)/settings/page.tsx -> /settings

// app/(auth)/layout.tsx
export default function AuthLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="max-w-md w-full">{children}</div>
    </div>
  );
}

// app/(dashboard)/layout.tsx
export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1">{children}</main>
    </div>
  );
}
```

---

## Server vs Client Components

### Server Components (Default)

```typescript
// Good: Server Components by default (no 'use client' directive)
// app/users/page.tsx
import { db } from '@/lib/db';
import { UserCard } from '@/components/users/user-card';

export default async function UsersPage() {
  const users = await db.user.findMany();

  return (
    <div>
      <h1>Users</h1>
      <div className="grid grid-cols-3 gap-4">
        {users.map((user) => (
          <UserCard key={user.id} user={user} />
        ))}
      </div>
    </div>
  );
}

// Good: Fetch data directly in Server Components
async function getUserData(id: string) {
  const res = await fetch(`https://api.example.com/users/${id}`, {
    cache: 'force-cache', // Cache by default
  });
  return res.json();
}

export default async function UserPage({ params }: { params: { id: string } }) {
  const user = await getUserData(params.id);

  return <UserProfile user={user} />;
}
```

### Client Components

```typescript
// Good: Use 'use client' for interactivity
// components/users/user-card.tsx
'use client';

import { useState } from 'react';

interface UserCardProps {
  user: User;
}

export function UserCard({ user }: UserCardProps) {
  const [liked, setLiked] = useState(false);

  return (
    <div className="card">
      <h3>{user.name}</h3>
      <button onClick={() => setLiked(!liked)}>
        {liked ? '❤️' : '🤍'}
      </button>
    </div>
  );
}

// Good: Client component with server actions
'use client';

import { deleteUser } from '@/app/actions/users';

export function UserActions({ userId }: { userId: string }) {
  const [isDeleting, setIsDeleting] = useState(false);

  const handleDelete = async () => {
    setIsDeleting(true);
    await deleteUser(userId);
    setIsDeleting(false);
  };

  return (
    <button onClick={handleDelete} disabled={isDeleting}>
      {isDeleting ? 'Deleting...' : 'Delete'}
    </button>
  );
}
```

### Mixing Server and Client Components

```typescript
// Good: Server Component with Client Component children
// app/users/page.tsx
import { UserListClient } from '@/components/users/user-list-client';

async function getUsers() {
  const res = await fetch('https://api.example.com/users', {
    next: { revalidate: 60 }, // Revalidate every 60 seconds
  });
  return res.json();
}

export default async function UsersPage() {
  const users = await getUsers();

  return <UserListClient initialUsers={users} />;
}

// components/users/user-list-client.tsx
'use client';

import { useState } from 'react';

interface UserListClientProps {
  initialUsers: User[];
}

export function UserListClient({ initialUsers }: UserListClientProps) {
  const [users, setUsers] = useState(initialUsers);
  const [filter, setFilter] = useState('');

  const filtered = users.filter((u) =>
    u.name.toLowerCase().includes(filter.toLowerCase())
  );

  return (
    <div>
      <input
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="Filter users..."
      />
      <ul>
        {filtered.map((user) => (
          <li key={user.id}>{user.name}</li>
        ))}
      </ul>
    </div>
  );
}
```

---

## Data Fetching Patterns

### Static Data Fetching

```typescript
// Good: Static data with revalidation
// app/products/page.tsx
export const revalidate = 3600; // Revalidate every hour

export default async function ProductsPage() {
  const products = await fetchProducts();

  return <ProductList products={products} />;
}

// Good: On-demand revalidation
// app/api/revalidate/route.ts
import { revalidatePath } from 'next/cache';
import { NextRequest, NextResponse } from 'next/server';

export async function POST(request: NextRequest) {
  const path = request.nextUrl.searchParams.get('path');

  if (path) {
    revalidatePath(path);
    return NextResponse.json({ revalidated: true });
  }

  return NextResponse.json({ revalidated: false });
}
```

### Dynamic Data Fetching

```typescript
// Good: Disable cache for real-time data
async function getLiveScore(gameId: string) {
  const res = await fetch(`https://api.example.com/scores/${gameId}`, {
    cache: 'no-store',
  });
  return res.json();
}

// Good: Conditional revalidation
async function getUserPosts(userId: string) {
  const res = await fetch(`https://api.example.com/users/${userId}/posts`, {
    next: {
      revalidate: 60,
      tags: [`posts-${userId}`],
    },
  });
  return res.json();
}
```

### Parallel Data Fetching

```typescript
// Good: Fetch data in parallel
export default async function DashboardPage() {
  const [user, posts, analytics] = await Promise.all([
    fetchUser(),
    fetchPosts(),
    fetchAnalytics(),
  ]);

  return <Dashboard user={user} posts={posts} analytics={analytics} />;
}

// Good: Use React Cache to prevent duplicate requests
import { cache } from 'react';

const getUser = cache(async (id: string) => {
  const res = await fetch(`https://api.example.com/users/${id}`);
  return res.json();
});

export default async function UserPage({ params }: { params: { id: string } }) {
  // Even if called multiple times, only fetches once
  const user = await getUser(params.id);
  const posts = await getUserPosts(params.id);

  return <UserProfile user={user} posts={posts} />;
}
```

---

## Server Actions

### Basic Server Actions

```typescript
// Good: Define server actions in separate files
// app/actions/users.ts
'use server';

import { db } from '@/lib/db';
import { revalidatePath } from 'next/cache';
import { redirect } from 'next/navigation';

export async function createUser(formData: FormData) {
  const name = formData.get('name') as string;
  const email = formData.get('email') as string;

  const user = await db.user.create({
    data: { name, email },
  });

  revalidatePath('/users');
  redirect(`/users/${user.id}`);
}

export async function updateUser(id: string, formData: FormData) {
  const name = formData.get('name') as string;
  const email = formData.get('email') as string;

  await db.user.update({
    where: { id },
    data: { name, email },
  });

  revalidatePath('/users');
  revalidatePath(`/users/${id}`);
}

export async function deleteUser(id: string) {
  await db.user.delete({
    where: { id },
  });

  revalidatePath('/users');
}
```

### Using Server Actions in Components

```typescript
// Good: Use server actions in client components
// components/users/create-user-form.tsx
'use client';

import { createUser } from '@/app/actions/users';
import { useFormStatus } from 'react-dom';

function SubmitButton() {
  const { pending } = useFormStatus();

  return (
    <button type="submit" disabled={pending}>
      {pending ? 'Creating...' : 'Create User'}
    </button>
  );
}

export function CreateUserForm() {
  return (
    <form action={createUser}>
      <input name="name" placeholder="Name" required />
      <input name="email" type="email" placeholder="Email" required />
      <SubmitButton />
    </form>
  );
}
```

### Server Actions with Error Handling

```typescript
// Good: Proper error handling in server actions
'use server';

import { z } from 'zod';
import { revalidatePath } from 'next/cache';

const createUserSchema = z.object({
  name: z.string().min(2),
  email: z.string().email(),
});

export async function createUser(prevState: any, formData: FormData) {
  const validatedFields = createUserSchema.safeParse({
    name: formData.get('name'),
    email: formData.get('email'),
  });

  if (!validatedFields.success) {
    return {
      errors: validatedFields.error.flatten().fieldErrors,
      message: 'Missing Fields. Failed to Create User.',
    };
  }

  const { name, email } = validatedFields.data;

  try {
    await db.user.create({ data: { name, email } });
    revalidatePath('/users');
    return { message: 'User created successfully' };
  } catch (error) {
    return { message: 'Database Error: Failed to Create User.' };
  }
}
```

---

## Routing and Navigation

### Programmatic Navigation

```typescript
// Good: Use redirect in Server Components
import { redirect } from 'next/navigation';

export default async function AdminPage() {
  const session = await getSession();

  if (!session?.isAdmin) {
    redirect('/login');
  }

  return <AdminDashboard />;
}

// Good: Use useRouter in Client Components
'use client';

import { useRouter } from 'next/navigation';

export function Navigation() {
  const router = useRouter();

  const handleClick = () => {
    router.push('/dashboard');
    router.refresh(); // Refresh current route data
  };

  return <button onClick={handleClick}>Go to Dashboard</button>;
}
```

### Dynamic Routes

```typescript
// Good: Dynamic route with params
// app/users/[id]/page.tsx
export default async function UserPage({
  params,
}: {
  params: { id: string };
}) {
  const user = await db.user.findUnique({
    where: { id: params.id },
  });

  if (!user) {
    notFound();
  }

  return <UserProfile user={user} />;
}

// Good: Catch-all routes
// app/docs/[...slug]/page.tsx
export default async function DocsPage({
  params,
}: {
  params: { slug: string[] };
}) {
  const path = params.slug.join('/');
  const doc = await getDocByPath(path);

  return <DocContent doc={doc} />;
}
```

### Middleware for Route Protection

```typescript
// middleware.ts
import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export function middleware(request: NextRequest) {
  const token = request.cookies.get('token')?.value;
  const { pathname } = request.nextUrl;

  // Protected routes
  if (pathname.startsWith('/dashboard') && !token) {
    return NextResponse.redirect(new URL('/login', request.url));
  }

  // Public routes
  return NextResponse.next();
}

export const config = {
  matcher: ['/dashboard/:path*', '/settings/:path*'],
};
```

---

## Performance Optimization

### Image Optimization

```typescript
// Good: Use next/image for automatic optimization
import Image from 'next/image';

export function UserAvatar({ user }: { user: User }) {
  return (
    <Image
      src={user.avatar}
      alt={user.name}
      width={100}
      height={100}
      priority={false} // Lazy load by default
      placeholder="blur"
      blurDataURL="/placeholder.jpg"
    />
  );
}

// Good: Remote images with loader
export function RemoteImage({ src, alt }: { src: string; alt: string }) {
  return (
    <Image
      src={src}
      alt={alt}
      width={800}
      height={600}
      loader={({ src, width, quality }) => {
        return `https://example.com/images/${src}?w=${width}&q=${quality || 75}`;
      }}
    />
  );
}
```

### Font Optimization

```typescript
// Good: Use next/font for font optimization
import { Inter, Roboto_Mono } from 'next/font/google';

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-inter',
  display: 'swap',
});

const robotoMono = Roboto_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
  display: 'swap',
});

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html className={`${inter.variable} ${robotoMono.variable}`}>
      <body>{children}</body>
    </html>
  );
}
```

### Code Splitting and Lazy Loading

```typescript
// Good: Lazy load heavy components
'use client';

import { lazy, Suspense } from 'react';

const ChartComponent = lazy(() => import('@/components/charts/Chart'));
const MapComponent = lazy(() => import('@/components/maps/Map'));

export function Dashboard() {
  return (
    <div>
      <Suspense fallback={<ChartSkeleton />}>
        <ChartComponent />
      </Suspense>
      <Suspense fallback={<MapSkeleton />}>
        <MapComponent />
      </Suspense>
    </div>
  );
}
```

---

## API Routes

### Route Handlers

```typescript
// Good: RESTful API route
// app/api/users/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { z } from 'zod';

const createUserSchema = z.object({
  name: z.string().min(2),
  email: z.string().email(),
});

export async function GET(request: NextRequest) {
  const { searchParams } = new URL(request.url);
  const page = parseInt(searchParams.get('page') || '1');
  const limit = parseInt(searchParams.get('limit') || '10');

  const users = await db.user.findMany({
    skip: (page - 1) * limit,
    take: limit,
  });

  return NextResponse.json({ users, page, limit });
}

export async function POST(request: NextRequest) {
  const body = await request.json();
  const validated = createUserSchema.safeParse(body);

  if (!validated.success) {
    return NextResponse.json(
      { errors: validated.error.errors },
      { status: 400 }
    );
  }

  const user = await db.user.create({
    data: validated.data,
  });

  return NextResponse.json(user, { status: 201 });
}
```

### Dynamic API Routes

```typescript
// Good: Dynamic route handlers
// app/api/users/[id]/route.ts
import { NextRequest, NextResponse } from 'next/server';

export async function GET(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const user = await db.user.findUnique({
    where: { id: params.id },
  });

  if (!user) {
    return NextResponse.json(
      { error: 'User not found' },
      { status: 404 }
    );
  }

  return NextResponse.json(user);
}

export async function PATCH(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  const body = await request.json();
  const user = await db.user.update({
    where: { id: params.id },
    data: body,
  });

  return NextResponse.json(user);
}

export async function DELETE(
  request: NextRequest,
  { params }: { params: { id: string } }
) {
  await db.user.delete({
    where: { id: params.id },
  });

  return NextResponse.json({ success: true });
}
```

---

## Authentication

### Using NextAuth.js

```typescript
// Good: Configure NextAuth
// app/api/auth/[...nextauth]/route.ts
import NextAuth from 'next-auth';
import Credentials from 'next-auth/providers/credentials';

export const { handlers, auth, signIn, signOut } = NextAuth({
  providers: [
    Credentials({
      credentials: {
        email: { label: 'Email', type: 'email' },
        password: { label: 'Password', type: 'password' },
      },
      async authorize(credentials) {
        const user = await db.user.findUnique({
          where: { email: credentials?.email as string },
        });

        if (!user || !user.password) {
          return null;
        }

        const isValid = await verifyPassword(
          credentials?.password as string,
          user.password
        );

        if (!isValid) {
          return null;
        }

        return {
          id: user.id,
          email: user.email,
          name: user.name,
        };
      },
    }),
  ],
  pages: {
    signIn: '/login',
    error: '/login',
  },
});

export const { GET, POST } = handlers;
```

### Protected Routes

```typescript
// Good: Middleware-based protection
// middleware.ts
export { auth as middleware } from '@/auth';

// auth.config.ts
import NextAuth from 'next-auth';
import { NextResponse } from 'next/server';

export default NextAuth({
  callbacks: {
    authorized({ request, auth }) {
      const { pathname } = request.nextUrl;

      if (pathname.startsWith('/dashboard')) {
        return !!auth;
      }

      return true;
    },
  },
});

export const config = {
  matcher: ['/dashboard/:path*'],
};
```

---

## State Management

### Server State (React Query)

```typescript
// Good: Use React Query for server state
// providers.tsx
'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 60 * 1000,
          },
        },
      })
  );

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
}

// hooks/use-users.ts
import { useQuery } from '@tanstack/react-query';

export function useUsers() {
  return useQuery({
    queryKey: ['users'],
    queryFn: async () => {
      const res = await fetch('/api/users');
      return res.json();
    },
  });
}
```

### URL State

```typescript
// Good: Use URL for shareable state
'use client';

import { useSearchParams, useRouter } from 'next/navigation';

export function ProductFilters() {
  const searchParams = useSearchParams();
  const router = useRouter();

  const category = searchParams.get('category') || 'all';
  const sort = searchParams.get('sort') || 'name';

  const updateFilter = (key: string, value: string) => {
    const params = new URLSearchParams(searchParams);
    params.set(key, value);
    router.push(`/?${params.toString()}`);
  };

  return (
    <div>
      <select
        value={category}
        onChange={(e) => updateFilter('category', e.target.value)}
      >
        <option value="all">All Categories</option>
        <option value="electronics">Electronics</option>
        <option value="clothing">Clothing</option>
      </select>

      <select
        value={sort}
        onChange={(e) => updateFilter('sort', e.target.value)}
      >
        <option value="name">Sort by Name</option>
        <option value="price">Sort by Price</option>
      </select>
    </div>
  );
}
```

---

## Common Patterns

### Layouts and Templates

```typescript
// Good: Root layout
// app/layout.tsx
import { Inter } from 'next/font/google';
import './globals.css';

const inter = Inter({ subsets: ['latin'] });

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className={inter.className}>
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}

// Good: Nested layouts
// app/(dashboard)/layout.tsx
export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="flex h-screen">
      <Sidebar />
      <main className="flex-1 overflow-auto">
        <Header />
        {children}
      </main>
    </div>
  );
}
```

### Loading States

```typescript
// Good: Loading UI with suspense
// app/users/loading.tsx
export default function Loading() {
  return (
    <div className="space-y-4">
      {[...Array(3)].map((_, i) => (
        <div key={i} className="h-20 bg-gray-200 animate-pulse rounded" />
      ))}
    </div>
  );
}

// Good: Suspense boundaries
// app/users/page.tsx
import { Suspense } from 'react';

export default async function UsersPage() {
  return (
    <div>
      <h1>Users</h1>
      <Suspense fallback={<UserListSkeleton />}>
        <UserList />
      </Suspense>
      <Suspense fallback={<StatsSkeleton />}>
        <UserStats />
      </Suspense>
    </div>
  );
}
```

---

## Anti-Patterns to Avoid

### Don't Use useEffect for Data Fetching in Server Components

```typescript
// Bad: Using useEffect in Server Component
export default function UsersPage() {
  const [users, setUsers] = useState([]);

  useEffect(() => {
    fetch('/api/users').then(r => r.json()).then(setUsers);
  }, []);

  return <UserList users={users} />;
}

// Good: Direct data fetching in Server Component
export default async function UsersPage() {
  const users = await db.user.findMany();

  return <UserList users={users} />;
}
```

### Don't Fetch the Same Data Multiple Times

```typescript
// Bad: Duplicate data fetching
export default async function DashboardPage() {
  const user = await fetchUser();
  const posts = await fetchUserPosts();
  const comments = await fetchUserComments();

  return <Dashboard user={user} posts={posts} comments={comments} />;
}

// Good: Use React cache
import { cache } from 'react';

const getUser = cache(async () => {
  return db.user.findFirst();
});

export default async function DashboardPage() {
  const user = await getUser(); // Fetches once
  const posts = await getUserPosts(user.id);
  const comments = await getUserComments(user.id);

  return <Dashboard user={user} posts={posts} comments={comments} />;
}
```

---

## Additional Resources

- [Next.js Documentation](https://nextjs.org/docs)
- [Next.js Learn](https://nextjs.org/learn)
- [Next.js App Router](https://nextjs.org/docs/app)
- [React Server Components](https://react.dev/reference/react/use-server)
- [Next.js GitHub](https://github.com/vercel/next.js)
