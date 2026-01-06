# React Style Guide

A comprehensive guide for building React applications with modern best practices, hooks, component patterns, and performance optimization (2025/2026).

## Table of Contents

- [Component Design Principles](#component-design-principles)
- [Component Patterns](#component-patterns)
- [React Hooks Best Practices](#react-hooks-best-practices)
- [State Management](#state-management)
- [Performance Optimization](#performance-optimization)
- [Data Fetching](#data-fetching)
- [TypeScript with React](#typescript-with-react)
- [Testing](#testing)
- [Styling Approaches](#styling-approaches)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Component Design Principles

### Single Responsibility Principle

```tsx
// Good: Component does one thing well
function UserAvatar({ userId, size }: UserAvatarProps) {
  const { data: user } = useUser(userId);
  return <img src={user?.avatarUrl} alt={user?.name} width={size} height={size} />;
}

// Good: Separate concerns into multiple components
function UserProfile({ userId }: UserProfileProps) {
  return (
    <div>
      <UserAvatar userId={userId} size={80} />
      <UserName userId={userId} />
      <UserEmail userId={userId} />
    </div>
  );
}

// Bad: Component doing too much
function UserProfile({ userId }: UserProfileProps) {
  const { data: user } = useUser(userId);
  return (
    <div>
      <img src={user?.avatarUrl} alt={user?.name} width={80} height={80} />
      <h1>{user?.name}</h1>
      <p>{user?.email}</p>
      <button onClick={() => sendEmail(user?.email)}>Send Email</button>
      <button onClick={() => editProfile(user)}>Edit</button>
    </div>
  );
}
```

### Component Naming

```tsx
// Good: Clear, descriptive component names
function UserList() { }
function UserListItem() { }
function UserListEmptyState() { }
function UserListError() { }

// Good: Prefix components with feature name
function AuthLoginForm() { }
function AuthRegisterForm() { }
function AuthForgotPassword() { }

// Bad: Vague or unclear names
function DataComponent() { }
function Thing() { }
function Component1() { }
```

### Props Interface Design

```tsx
// Good: Descriptive prop names
interface UserCardProps {
  userId: string;
  showEmail?: boolean;
  onEdit?: (user: User) => void;
  className?: string;
}

// Good: Use discriminated unions for mutually exclusive props
type ButtonProps =
  | { variant: 'primary'; onClick: () => void }
  | { variant: 'secondary'; href: string };

// Bad: Unclear prop names
interface UserCardProps {
  id: string;
  se?: boolean;
  o: (u: User) => void;
  cls?: string;
}
```

---

## Component Patterns

### Functional Components with Hooks

```tsx
// Good: Modern functional component with hooks
function UserList({ filter }: UserListProps) {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    async function fetchUsers() {
      setLoading(true);
      setError(null);
      try {
        const data = await api.getUsers(filter);
        setUsers(data);
      } catch (err) {
        setError(err as Error);
      } finally {
        setLoading(false);
      }
    }
    fetchUsers();
  }, [filter]);

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;
  if (users.length === 0) return <EmptyState />;

  return (
    <ul>
      {users.map((user) => (
        <UserListItem key={user.id} user={user} />
      ))}
    </ul>
  );
}

// Avoid: Class components (legacy pattern)
class UserList extends React.Component<UserListProps, UserListState> {
  // Legacy pattern - prefer functional components
}
```

### Compound Components Pattern

```tsx
// Good: Compound components for flexible composition
function Tabs({ children }: TabsProps) {
  const [activeTab, setActiveTab] = useState(0);

  return (
    <TabsContext.Provider value={{ activeTab, setActiveTab }}>
      <div className="tabs">{children}</div>
    </TabsContext.Provider>
  );
}

Tabs.TabList = function TabList({ children }: TabListProps) {
  return <div className="tab-list">{children}</div>;
};

Tabs.Tab = function Tab({ index, children }: TabProps) {
  const { activeTab, setActiveTab } = useTabs();
  const isActive = activeTab === index;

  return (
    <button
      className={isActive ? 'tab active' : 'tab'}
      onClick={() => setActiveTab(index)}
    >
      {children}
    </button>
  );
};

Tabs.TabPanel = function TabPanel({ index, children }: TabPanelProps) {
  const { activeTab } = useTabs();

  if (activeTab !== index) return null;
  return <div className="tab-panel">{children}</div>;
};

// Usage
<Tabs>
  <Tabs.TabList>
    <Tabs.Tab index={0}>Home</Tabs.Tab>
    <Tabs.Tab index={1}>Profile</Tabs.Tab>
    <Tabs.Tab index={2}>Settings</Tabs.Tab>
  </Tabs.TabList>
  <Tabs.TabPanel index={0}>Home Content</Tabs.TabPanel>
  <Tabs.TabPanel index={1}>Profile Content</Tabs.TabPanel>
  <Tabs.TabPanel index={2}>Settings Content</Tabs.TabPanel>
</Tabs>
```

### Render Props Pattern

```tsx
// Good: Render props for flexible rendering
function DataSource({ render, getData }: DataSourceProps) {
  const [data, setData] = useState<Data | null>(null);

  useEffect(() => {
    getData().then(setData);
  }, [getData]);

  return render(data);
}

// Usage
<DataSource
  getData={() => api.getUsers()}
  render={(users) => (
    <UserList users={users} />
  )}
/>

// Alternative: Use children as render function
<DataSource getData={() => api.getUsers()}>
  {(users) => <UserList users={users} />}
</DataSource>
```

### Higher-Order Components (HOC) Pattern

```tsx
// Good: HOC for cross-cutting concerns
function withLoading<P extends object>(
  Component: ComponentType<P>,
  loadingCondition: (props: P) => boolean
) {
  return function WithLoading(props: P) {
    if (loadingCondition(props)) {
      return <LoadingSpinner />;
    }
    return <Component {...props} />;
  };
}

// Usage
const UserListWithLoading = withLoading(UserList, (props) => props.isLoading);

// Note: Prefer custom hooks over HOCs in modern React
function useUserList(filter: string) {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(false);

  // Implementation...

  return { users, loading };
}
```

---

## React Hooks Best Practices

### useState Guidelines

```tsx
// Good: Use useState with explicit typing
function UserProfile() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Good: Functional updates for derived state
  const increment = () => setCount(prev => prev + 1);

  // Good: Lazy initialization
  const [data, setData] = useState(() => {
    const initial = getInitialValue();
    return initial;
  });

  return <div>{/* ... */}</div>;
}

// Good: Group related state
function useForm<T extends Record<string, unknown>>(initial: T) {
  const [values, setValues] = useState<T>(initial);
  const [errors, setErrors] = useState<Record<keyof T, string>>({} as any);
  const [touched, setTouched] = useState<Record<keyof T, boolean>>({} as any);

  // Implementation...
}
```

### useEffect Guidelines

```tsx
// Good: Specify all dependencies
function UserProfile({ userId }: UserProfileProps) {
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function fetchUser() {
      const data = await api.getUser(userId);
      if (!cancelled) {
        setUser(data);
      }
    }

    fetchUser();

    return () => {
      cancelled = true;
    };
  }, [userId]); // All dependencies specified

  return <div>{/* ... */}</div>;
}

// Good: Separate effects for different concerns
function UserProfile({ userId }: UserProfileProps) {
  // Fetch user data
  useEffect(() => {
    api.getUser(userId).then(setUser);
  }, [userId]);

  // Set up event listeners
  useEffect(() => {
    const handleResize = () => {
      // Handle resize
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Track page views
  useEffect(() => {
    analytics.track('User Profile Viewed', { userId });
  }, [userId]);

  return <div>{/* ... */}</div>;
}

// Good: Custom hooks to encapsulate effect logic
function useUser(userId: string) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function fetchUser() {
      setLoading(true);
      setError(null);
      try {
        const data = await api.getUser(userId);
        if (!cancelled) {
          setUser(data);
        }
      } catch (err) {
        if (!cancelled) {
          setError(err as Error);
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    fetchUser();

    return () => {
      cancelled = true;
    };
  }, [userId]);

  return { user, loading, error };
}
```

### useContext Best Practices

```tsx
// Good: Create context with explicit typing
interface UserContextType {
  user: User | null;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

const UserContext = createContext<UserContextType | undefined>(undefined);

// Good: Create custom hook for using context
function useUser() {
  const context = useContext(UserContext);
  if (!context) {
    throw new Error('useUser must be used within UserProvider');
  }
  return context;
}

// Good: Provider component
function UserProvider({ children }: UserProviderProps) {
  const [user, setUser] = useState<User | null>(null);

  const login = async (email: string, password: string) => {
    const user = await api.login(email, password);
    setUser(user);
  };

  const logout = () => {
    setUser(null);
  };

  const value = useMemo(
    () => ({ user, login, logout }),
    [user]
  );

  return <UserContext.Provider value={value}>{children}</UserContext.Provider>;
}

// Usage in components
function UserProfile() {
  const { user, logout } = useUser();
  return <div>{/* ... */}</div>;
}
```

### useCallback Guidelines

```tsx
// Good: Use useCallback to memoize callbacks
function UserList({ onUserSelect }: UserListProps) {
  const [users, setUsers] = useState<User[]>([]);

  const handleUserClick = useCallback(
    (user: User) => {
      onUserSelect(user);
    },
    [onUserSelect]
  );

  return (
    <ul>
      {users.map((user) => (
        <li key={user.id}>
          <UserCard user={user} onClick={handleUserClick} />
        </li>
      ))}
    </ul>
  );
}

// Good: Use useCallback for event handlers passed to children
function ParentComponent() {
  const [count, setCount] = useState(0);

  const handleClick = useCallback(() => {
    setCount((c) => c + 1);
  }, []);

  const handleReset = useCallback(() => {
    setCount(0);
  }, []);

  return (
    <div>
      <ChildComponent onClick={handleClick} onReset={handleReset} />
    </div>
  );
}

// Good: Memoize event handlers with dependencies
function SearchForm({ onSearch }: SearchFormProps) {
  const [query, setQuery] = useState('');

  const handleSubmit = useCallback(
    (e: FormEvent) => {
      e.preventDefault();
      onSearch(query);
    },
    [onSearch, query]
  );

  return (
    <form onSubmit={handleSubmit}>
      <input value={query} onChange={(e) => setQuery(e.target.value)} />
      <button type="submit">Search</button>
    </form>
  );
}
```

### useMemo Guidelines

```tsx
// Good: Use useMemo for expensive computations
function ProductList({ products, filter }: ProductListProps) {
  const filteredProducts = useMemo(() => {
    return products.filter((p) =>
      p.name.toLowerCase().includes(filter.toLowerCase())
    );
  }, [products, filter]);

  const sortedProducts = useMemo(() => {
    return filteredProducts.sort((a, b) => a.price - b.price);
  }, [filteredProducts]);

  return <div>{/* ... */}</div>;
}

// Good: Use useMemo for complex objects
function DataTable({ data }: DataTableProps) {
  const columns = useMemo(
    () => [
      { key: 'id', label: 'ID', sortable: true },
      { key: 'name', label: 'Name', sortable: true },
      { key: 'email', label: 'Email', sortable: false },
    ],
    []
  );

  const pagination = useMemo(
    () => ({
      page: 1,
      pageSize: 10,
      total: data.length,
    }),
    [data.length]
  );

  return <Table columns={columns} data={data} pagination={pagination} />;
}

// Good: Don't premature optimize - only useMemo when necessary
function SimpleComponent({ items }: { items: string[] }) {
  // Not expensive enough to warrant useMemo
  const total = items.length;

  return <div>Total: {total}</div>;
}
```

### Custom Hooks Best Practices

```tsx
// Good: Custom hook for reusable logic
function useLocalStorage<T>(key: string, initialValue: T) {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = window.localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch (error) {
      return initialValue;
    }
  });

  const setValue = useCallback(
    (value: T | ((val: T) => T)) => {
      try {
        const valueToStore = value instanceof Function ? value(storedValue) : value;
        setStoredValue(valueToStore);
        window.localStorage.setItem(key, JSON.stringify(valueToStore));
      } catch (error) {
        console.error(error);
      }
    },
    [key, storedValue]
  );

  return [storedValue, setValue] as const;
}

// Good: Custom hook for API calls
function useApi<T>(url: string) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const fetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch(url);
      const data = await response.json();
      setData(data);
    } catch (err) {
      setError(err as Error);
    } finally {
      setLoading(false);
    }
  }, [url]);

  useEffect(() => {
    fetch();
  }, [fetch]);

  return { data, loading, error, refetch: fetch };
}

// Good: Compose custom hooks
function useUserPreferences(userId: string) {
  const { data: user } = useApi<User>(`/api/users/${userId}`);
  const [preferences, setPreferences] = useLocalStorage<Preferences>(
    `preferences-${userId}`,
    {}
  );

  return { user, preferences, setPreferences };
}
```

---

## State Management

### Local State vs Global State

```tsx
// Good: Use local state for component-specific data
function UserForm() {
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    api.createUser({ name, email });
  };

  return <form onSubmit={handleSubmit}>{/* ... */}</form>;
}

// Good: Use global state for shared application data
// Using Zustand
const useUserStore = create<UserStore>((set) => ({
  user: null,
  login: (user) => set({ user }),
  logout: () => set({ user: null }),
}));

// Using Context API
const AuthContext = createContext<AuthContextType | undefined>(undefined);

function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);

  const login = useCallback(async (email: string, password: string) => {
    const user = await api.login(email, password);
    setUser(user);
  }, []);

  const logout = useCallback(() => {
    setUser(null);
  }, []);

  const value = useMemo(
    () => ({ user, login, logout }),
    [user, login, logout]
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}
```

### State Reduction Pattern

```tsx
// Good: Reduce related state into a single state object
function UserForm() {
  const [form, setForm] = useState({
    name: '',
    email: '',
    age: 0,
    newsletter: false,
  });

  const handleChange = useCallback((field: keyof typeof form) => (
    e: ChangeEvent<HTMLInputElement>
  ) => {
    const value = e.target.type === 'checkbox'
      ? e.target.checked
      : e.target.value;
    setForm((prev) => ({ ...prev, [field]: value }));
  }, []);

  return (
    <form>
      <input
        value={form.name}
        onChange={handleChange('name')}
        placeholder="Name"
      />
      <input
        value={form.email}
        onChange={handleChange('email')}
        placeholder="Email"
      />
      <input
        type="number"
        value={form.age}
        onChange={handleChange('age')}
        placeholder="Age"
      />
      <label>
        <input
          type="checkbox"
          checked={form.newsletter}
          onChange={handleChange('newsletter')}
        />
        Subscribe to newsletter
      </label>
    </form>
  );
}
```

---

## Performance Optimization

### Code Splitting and Lazy Loading

```tsx
// Good: Lazy load routes
const Home = lazy(() => import('./pages/Home'));
const About = lazy(() => import('./pages/About'));
const Dashboard = lazy(() => import('./pages/Dashboard'));

function App() {
  return (
    <Suspense fallback={<PageLoader />}>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/about" element={<About />} />
        <Route path="/dashboard" element={<Dashboard />} />
      </Routes>
    </Suspense>
  );
}

// Good: Lazy load heavy components
function UserProfile() {
  const [showChart, setShowChart] = useState(false);

  const ChartComponent = useMemo(
    () => lazy(() => import('./components/ChartComponent')),
    []
  );

  return (
    <div>
      <button onClick={() => setShowChart(true)}>Show Chart</button>
      {showChart && (
        <Suspense fallback={<ChartSkeleton />}>
          <ChartComponent />
        </Suspense>
      )}
    </div>
  );
}
```

### Memoization Strategies

```tsx
// Good: Use React.memo for expensive components
const UserCard = memo(function UserCard({ user, onClick }: UserCardProps) {
  return (
    <div onClick={() => onClick(user)}>
      <img src={user.avatar} alt={user.name} />
      <h3>{user.name}</h3>
      <p>{user.email}</p>
    </div>
  );
});

// Good: Use comparison function for custom memoization
const UserCard = memo(
  function UserCard({ user, onClick }: UserCardProps) {
    return <div>{/* ... */}</div>;
  },
  (prevProps, nextProps) => {
    return prevProps.user.id === nextProps.user.id &&
           prevProps.onClick === nextProps.onClick;
  }
);

// Good: Use useMemo for expensive computations
function ProductList({ products }: ProductListProps) {
  const expensiveValue = useMemo(() => {
    return computeExpensiveValue(products);
  }, [products]);

  return <div>{expensiveValue}</div>;
}

// Good: Virtualize long lists
import { useVirtualizer } from '@tanstack/react-virtual';

function VirtualList({ items }: VirtualListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const rowVirtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 50,
  });

  return (
    <div ref={parentRef} style={{ height: '400px', overflow: 'auto' }}>
      <div style={{ height: `${rowVirtualizer.getTotalSize()}px` }}>
        {rowVirtualizer.getVirtualItems().map((virtualRow) => (
          <div
            key={virtualRow.key}
            style={{
              position: 'absolute',
              top: 0,
              left: 0,
              width: '100%',
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            {items[virtualRow.index]}
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## Data Fetching

### Using React Query

```tsx
// Good: Use React Query for server state
function useUsers(filter: string) {
  return useQuery({
    queryKey: ['users', filter],
    queryFn: () => api.getUsers(filter),
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}

function UserList() {
  const { data: users, isLoading, error } = useUsers('');

  if (isLoading) return <LoadingSpinner />;
  if (error) return <ErrorMessage error={error} />;

  return (
    <ul>
      {users?.map((user) => (
        <UserListItem key={user.id} user={user} />
      ))}
    </ul>
  );
}

// Good: Use mutations for writes
function useUpdateUser() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (user: User) => api.updateUser(user),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: ['users'] });
      toast.success('User updated successfully');
    },
    onError: (error) => {
      toast.error('Failed to update user');
    },
  });
}

function UserForm({ user }: UserFormProps) {
  const updateUser = useUpdateUser();

  const handleSubmit = (data: FormData) => {
    updateUser.mutate({ ...user, ...data });
  };

  return <form onSubmit={handleSubmit}>{/* ... */}</form>;
}
```

### Server Components (Next.js)

```tsx
// Good: Server Components for data fetching
async function UserList() {
  const users = await api.getUsers();

  return (
    <ul>
      {users.map((user) => (
        <UserListItem key={user.id} user={user} />
      ))}
    </ul>
  );
}

// Good: Client Components for interactivity
'use client';

function UserListItem({ user }: UserListItemProps) {
  const [liked, setLiked] = useState(false);

  return (
    <li>
      <span>{user.name}</span>
      <button onClick={() => setLiked(!liked)}>
        {liked ? '❤️' : '🤍'}
      </button>
    </li>
  );
}
```

---

## TypeScript with React

### Typing Props

```tsx
// Good: Explicit prop types
interface UserCardProps {
  user: User;
  onEdit?: (user: User) => void;
  onDelete?: (userId: string) => void;
  className?: string;
  children?: ReactNode;
}

function UserCard({ user, onEdit, onDelete, className, children }: UserCardProps) {
  return (
    <div className={className}>
      <h3>{user.name}</h3>
      {children}
      {onEdit && <button onClick={() => onEdit(user)}>Edit</button>}
      {onDelete && <button onClick={() => onDelete(user.id)}>Delete</button>}
    </div>
  );
}

// Good: Generic component types
interface ListProps<T> {
  items: T[];
  renderItem: (item: T) => ReactNode;
  keyExtractor: (item: T) => string;
}

function List<T>({ items, renderItem, keyExtractor }: ListProps<T>) {
  return (
    <ul>
      {items.map((item) => (
        <li key={keyExtractor(item)}>{renderItem(item)}</li>
      ))}
    </ul>
  );
}

// Usage
<List
  items={users}
  renderItem={(user) => <UserCard user={user} />}
  keyExtractor={(user) => user.id}
/>
```

### Typing Hooks

```tsx
// Good: Type custom hooks
function useApi<T>(url: string): UseApiResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  // Implementation...

  return { data, loading, error };
}

interface UseApiResult<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
}

// Good: Type event handlers
function SearchForm() {
  const [query, setQuery] = useState('');

  const handleSubmit = (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    // Search logic
  };

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    setQuery(e.target.value);
  };

  return (
    <form onSubmit={handleSubmit}>
      <input value={query} onChange={handleChange} />
    </form>
  );
}
```

---

## Testing

### Component Testing with React Testing Library

```tsx
// Good: Test user behavior, not implementation
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { UserForm } from './UserForm';

describe('UserForm', () => {
  it('should submit form with valid data', async () => {
    const onSubmit = jest.fn();
    render(<UserForm onSubmit={onSubmit} />);

    const nameInput = screen.getByLabelText('Name');
    const emailInput = screen.getByLabelText('Email');
    const submitButton = screen.getByRole('button', { name: 'Submit' });

    fireEvent.change(nameInput, { target: { value: 'John Doe' } });
    fireEvent.change(emailInput, { target: { value: 'john@example.com' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledWith({
        name: 'John Doe',
        email: 'john@example.com',
      });
    });
  });

  it('should show validation errors for invalid email', () => {
    render(<UserForm onSubmit={jest.fn()} />);

    const emailInput = screen.getByLabelText('Email');
    fireEvent.change(emailInput, { target: { value: 'invalid-email' } });
    fireEvent.blur(emailInput);

    expect(screen.getByText('Please enter a valid email')).toBeInTheDocument();
  });
});
```

### Hook Testing

```tsx
// Good: Test custom hooks
import { renderHook, act } from '@testing-library/react';
import { useCounter } from './useCounter';

describe('useCounter', () => {
  it('should increment counter', () => {
    const { result } = renderHook(() => useCounter());

    act(() => {
      result.current.increment();
    });

    expect(result.current.count).toBe(1);
  });

  it('should decrement counter', () => {
    const { result } = renderHook(() => useCounter());

    act(() => {
      result.current.decrement();
    });

    expect(result.current.count).toBe(-1);
  });
});
```

---

## Styling Approaches

### CSS Modules

```tsx
// Good: CSS Modules for component-scoped styles
import styles from './UserCard.module.css';

function UserCard({ user }: UserCardProps) {
  return (
    <div className={styles.card}>
      <img src={user.avatar} alt={user.name} className={styles.avatar} />
      <h3 className={styles.name}>{user.name}</h3>
      <p className={styles.email}>{user.email}</p>
    </div>
  );
}
```

### Tailwind CSS

```tsx
// Good: Utility-first approach with Tailwind
function UserCard({ user }: UserCardProps) {
  return (
    <div className="bg-white rounded-lg shadow-md p-4 hover:shadow-lg transition-shadow">
      <img
        src={user.avatar}
        alt={user.name}
        className="w-16 h-16 rounded-full mx-auto"
      />
      <h3 className="text-lg font-semibold text-center mt-2">{user.name}</h3>
      <p className="text-sm text-gray-600 text-center">{user.email}</p>
    </div>
  );
}
```

### Styled Components

```tsx
// Good: CSS-in-JS with styled-components
import styled from 'styled-components';

const Card = styled.div`
  background: white;
  border-radius: 8px;
  padding: 16px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);

  &:hover {
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
  }
`;

const Avatar = styled.img`
  width: 64px;
  height: 64px;
  border-radius: 50%;
  margin: 0 auto;
`;

function UserCard({ user }: UserCardProps) {
  return (
    <Card>
      <Avatar src={user.avatar} alt={user.name} />
      <h3>{user.name}</h3>
      <p>{user.email}</p>
    </Card>
  );
}
```

---

## Common Patterns

### Controlled Components

```tsx
// Good: Controlled input components
function Input({ value, onChange, label }: InputProps) {
  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    onChange(e.target.value);
  };

  return (
    <div>
      <label>{label}</label>
      <input value={value} onChange={handleChange} />
    </div>
  );
}

// Usage
function SearchForm() {
  const [query, setQuery] = useState('');

  return (
    <form>
      <Input
        label="Search"
        value={query}
        onChange={setQuery}
      />
    </form>
  );
}
```

### Error Boundaries

```tsx
// Good: Error boundaries for graceful error handling
class ErrorBoundary extends React.Component<
  { children: ReactNode; fallback: ReactNode },
  { hasError: boolean }
> {
  constructor(props: any) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Error caught by boundary:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback;
    }
    return this.props.children;
  }
}

// Usage
function App() {
  return (
    <ErrorBoundary fallback={<ErrorFallback />}>
      <MyComponent />
    </ErrorBoundary>
  );
}
```

---

## Anti-Patterns to Avoid

### Don't Mutate State Directly

```tsx
// Bad: Direct mutation
const [users, setUsers] = useState<User[]>([]);
users.push(newUser); // Wrong!

// Bad: Direct mutation of nested state
const [user, setUser] = useState<User>({ name: '', email: '' });
user.name = 'John'; // Wrong!

// Good: Create new objects/arrays
setUsers([...users, newUser]);
setUsers(users.map(u => u.id === newUser.id ? newUser : u));

// Good: Create new objects for nested state
setUser({ ...user, name: 'John' });
setUser((prev) => ({ ...prev, name: 'John' }));
```

### Don't Use useEffect for Everything

```tsx
// Bad: Using useEffect for derived state
function UserProfile({ userId }: UserProfileProps) {
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    setUser(getUserById(userId));
  }, [userId]);

  return <div>{user?.name}</div>;
}

// Good: Derive directly during render
function UserProfile({ userId }: UserProfileProps) {
  const user = getUserById(userId);
  return <div>{user?.name}</div>;
}

// Good: Use useEffect for side effects only
function UserProfile({ userId }: UserProfileProps) {
  const user = getUserById(userId);

  useEffect(() => {
    analytics.track('User Profile Viewed', { userId });
  }, [userId]);

  return <div>{user?.name}</div>;
}
```

### Don't Over-Optimize

```tsx
// Bad: Unnecessary memoization
function SimpleComponent({ name }: { name: string }) {
  const memoizedName = useMemo(() => name, [name]);
  return <div>{memoizedName}</div>;
}

// Good: Keep it simple
function SimpleComponent({ name }: { name: string }) {
  return <div>{name}</div>;
}
```

---

## Additional Resources

- [React Documentation](https://react.dev)
- [React Hooks FAQ](https://react.dev/reference/react)
- [React Query Documentation](https://tanstack.com/query/latest)
- [Testing Library Documentation](https://testing-library.com/docs/react-testing-library/intro/)
- [React TypeScript Cheatsheet](https://react-typescript-cheatsheet.netlify.app/)
