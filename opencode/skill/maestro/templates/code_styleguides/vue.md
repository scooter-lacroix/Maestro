# Vue.js Style Guide

A comprehensive guide for building Vue.js applications using Composition API, TypeScript, and modern best practices (2025/2026).

## Table of Contents

- [Component Design](#component-design)
- [Composition API](#composition-api)
- [Reactivity System](#reactivity-system)
- [Props and Emits](#props-and-emits)
- [State Management](#state-management)
- [Routing](#routing)
- [Performance Optimization](#performance-optimization)
- [Testing](#testing)
- [TypeScript Integration](#typescript-integration)
- [Common Patterns](#common-patterns)
- [Anti-Patterns to Avoid](#anti-patterns-to-avoid)

---

## Component Design

### Single File Component Structure

```vue
<!-- Good: Organized SFC structure -->
<script setup lang="ts">
// 1. Imports
import { ref, computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { useUserStore } from '@/stores/user';
import type { User } from '@/types';

// 2. Props definition
interface Props {
  userId: string;
  showEmail?: boolean;
}
const props = withDefaults(defineProps<Props>(), {
  showEmail: false,
});

// 3. Emits definition
interface Emits {
  (e: 'update', user: User): void;
  (e: 'delete', userId: string): void;
}
const emit = defineEmits<Emits>();

// 4. Composables
const router = useRouter();
const userStore = useUserStore();

// 5. Reactive state
const user = ref<User | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

// 6. Computed properties
const displayName = computed(() => {
  return user.value?.name ?? 'Unknown';
});

const initials = computed(() => {
  return displayName.value
    .split(' ')
    .map(n => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
});

// 7. Methods
const fetchUser = async () => {
  loading.value = true;
  error.value = null;
  try {
    user.value = await userStore.getUserById(props.userId);
  } catch (err) {
    error.value = 'Failed to fetch user';
  } finally {
    loading.value = false;
  }
};

const handleUpdate = () => {
  if (user.value) {
    emit('update', user.value);
  }
};

const handleDelete = () => {
  emit('delete', props.userId);
};

// 8. Lifecycle hooks
onMounted(() => {
  fetchUser();
});
</script>

<template>
  <div class="user-card">
    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <div v-else-if="user" class="user-info">
      <div class="avatar">{{ initials }}</div>
      <h3>{{ displayName }}</h3>
      <p v-if="showEmail && user.email">{{ user.email }}</p>
      <button @click="handleUpdate">Update</button>
      <button @click="handleDelete">Delete</button>
    </div>
  </div>
</template>

<style scoped>
.user-card {
  @apply rounded-lg border p-4 shadow-sm;
}

.avatar {
  @apply flex h-12 w-12 items-center justify-center rounded-full bg-blue-500 text-white;
}
</style>
```

### Component Naming

```vue
<!-- Good: Multi-word component names -->
<UserProfile />
<UserAvatar />
<UserListItem />

<!-- Bad: Single-word names -->
<User />
<Profile />

<!-- Good: PascalCase for components -->
<script setup lang="ts">
// UserProfile.vue
import UserCard from './UserCard.vue';
import UserAvatar from './UserAvatar.vue';
</script>

<template>
  <div>
    <UserCard />
    <UserAvatar />
  </div>
</template>
```

---

## Composition API

### Composables

```typescript
// Good: Reusable composables
// composables/useUser.ts
import { ref, computed } from 'vue';
import { useUserStore } from '@/stores/user';
import type { User } from '@/types';

export function useUser(userId: string) {
  const userStore = useUserStore();

  const user = ref<User | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const fetchUser = async () => {
    loading.value = true;
    error.value = null;
    try {
      user.value = await userStore.getUserById(userId);
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch user';
    } finally {
      loading.value = false;
    }
  };

  const updateProfile = async (data: Partial<User>) => {
    if (!user.value) return;

    loading.value = true;
    error.value = null;
    try {
      user.value = await userStore.updateUser(user.value.id, data);
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to update user';
    } finally {
      loading.value = false;
    }
  };

  const isAuthenticated = computed(() => !!user.value);
  const fullName = computed(() => {
    if (!user.value) return '';
    return `${user.value.firstName} ${user.value.lastName}`.trim();
  });

  return {
    user,
    loading,
    error,
    isAuthenticated,
    fullName,
    fetchUser,
    updateProfile,
  };
}

// Usage in component
<script setup lang="ts">
import { useUser } from '@/composables/useUser';

const props = defineProps<{ userId: string }>();

const { user, loading, error, fullName, updateProfile } = useUser(props.userId);

onMounted(() => {
  fetchUser();
});
</script>
```

### Lifecycle Hooks

```vue
<script setup lang="ts">
import {
  onMounted,
  onBeforeMount,
  onUpdated,
  onBeforeUnmount,
  onUnmounted,
} from 'vue';

// Good: Use lifecycle hooks for setup/teardown
onBeforeMount(() => {
  console.log('Component is about to mount');
});

onMounted(() => {
  // Fetch data, set up event listeners
  document.addEventListener('keydown', handleKeydown);
});

onUpdated(() => {
  // Called after any data change
});

onBeforeUnmount(() => {
  // Cleanup before unmount
});

onUnmounted(() => {
  // Final cleanup
  document.removeEventListener('keydown', handleKeydown);
});

// Good: Use watchEffect for automatic tracking
import { watchEffect, watch } from 'vue';

watchEffect(() => {
  // Automatically tracks reactive dependencies
  console.log('Count is:', count.value);
});

// Good: Use watch for specific sources with getters
watch(
  () => props.userId,
  (newId, oldId) => {
    console.log(`User ID changed from ${oldId} to ${newId}`);
    fetchUser(newId);
  }
);

// Good: Watch multiple sources
watch([props.userId, props.includeInactive], ([id, include]) => {
  fetchUsers(id, include);
});
</script>
```

---

## Reactivity System

### Ref vs Reactive

```typescript
// Good: Use ref for primitives
const count = ref(0);
const message = ref('Hello');
const isActive = ref(false);

// Good: Use reactive for objects
const user = reactive<User>({
  id: '1',
  name: 'John',
  email: 'john@example.com',
});

// Good: Use ref with objects for reassignment
const user = ref<User | null>(null);

const fetchUser = async () => {
  user.value = await api.getUser(); // Can replace entire object
};

// Good: Use toRefs when destructuring reactive
import { toRefs } from 'vue';

const state = reactive({
  count: 0,
  message: 'Hello',
});

const { count, message } = toRefs(state);

// Good: Computed for derived state
const doubleCount = computed(() => count.value * 2);
const greeting = computed(() => `${message.value}!`);

// Good: Writable computed
const fullName = computed({
  get: () => `${firstName.value} ${lastName.value}`,
  set: (value: string) => {
    [firstName.value, lastName.value] = value.split(' ');
  },
});
```

### Watch and WatchEffect

```typescript
// Good: Use watchEffect for automatic dependency tracking
watchEffect(() => {
  console.log('Count changed:', count.value);
});

// Good: Use watch for specific sources
watch(count, (newVal, oldVal) => {
  console.log(`Count changed from ${oldVal} to ${newVal}`);
});

// Good: Watch with immediate option
watch(
  () => props.userId,
  (userId) => {
    fetchUser(userId);
  },
  { immediate: true }
);

// Good: Watch multiple sources
watch(
  [firstName, lastName],
  ([first, last]) => {
    console.log(`Name changed to ${first} ${last}`);
  }
);

// Good: Watch deep objects
const user = reactive({ profile: { name: 'John' } });

watch(
  () => user,
  (user) => {
    console.log('User changed:', user);
  },
  { deep: true }
);
```

---

## Props and Emits

### Type-Safe Props

```vue
<script setup lang="ts">
// Good: Interface-based props with TypeScript
interface Props {
  user: User;
  showAvatar?: boolean;
  size?: 'small' | 'medium' | 'large';
}

const props = withDefaults(defineProps<Props>(), {
  showAvatar: true,
  size: 'medium',
});

// Good: Props validation
const props = defineProps({
  userId: {
    type: String,
    required: true,
    validator: (value: string) => {
      return value.length > 0;
    },
  },
  size: {
    type: String as PropType<'small' | 'medium' | 'large'>,
    default: 'medium',
  },
});
</script>
```

### Type-Safe Emits

```vue
<script setup lang="ts">
// Good: Interface-based emits with TypeScript
interface Emits {
  (e: 'update', value: string): void;
  (e: 'delete', id: string): void;
  (e: 'change', payload: { id: string; value: number }): void;
}

const emit = defineEmits<Emits>();

// Usage
const handleUpdate = () => {
  emit('update', 'new value');
};

const handleDelete = () => {
  emit('delete', props.userId);
};

const handleChange = () => {
  emit('change', { id: '1', value: 42 });
};
</script>
```

---

## State Management

### Pinia Stores

```typescript
// Good: Pinia store with TypeScript
// stores/user.ts
import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import type { User } from '@/types';

export const useUserStore = defineStore('user', () => {
  // State
  const user = ref<User | null>(null);
  const users = ref<User[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // Getters
  const isAuthenticated = computed(() => !!user.value);
  const userCount = computed(() => users.value.length);

  // Actions
  async function fetchUsers() {
    loading.value = true;
    error.value = null;
    try {
      const response = await fetch('/api/users');
      users.value = await response.json();
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch users';
    } finally {
      loading.value = false;
    }
  }

  async function getUserById(id: string): Promise<User> {
    const cached = users.value.find(u => u.id === id);
    if (cached) return cached;

    const response = await fetch(`/api/users/${id}`);
    return response.json();
  }

  async function createUser(data: CreateUserDto): Promise<User> {
    const response = await fetch('/api/users', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });

    const user = await response.json();
    users.value.push(user);
    return user;
  }

  async function updateUser(id: string, data: UpdateUserDto): Promise<User> {
    const response = await fetch(`/api/users/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });

    const updated = await response.json();
    const index = users.value.findIndex(u => u.id === id);
    if (index !== -1) {
      users.value[index] = updated;
    }

    return updated;
  }

  async function deleteUser(id: string): Promise<void> {
    await fetch(`/api/users/${id}`, { method: 'DELETE' });
    users.value = users.value.filter(u => u.id !== id);
  }

  function setCurrentUser(user: User | null) {
    user.value = user;
  }

  function $reset() {
    user.value = null;
    users.value = [];
    loading.value = false;
    error.value = null;
  }

  return {
    // State
    user,
    users,
    loading,
    error,
    // Getters
    isAuthenticated,
    userCount,
    // Actions
    fetchUsers,
    getUserById,
    createUser,
    updateUser,
    deleteUser,
    setCurrentUser,
    $reset,
  };
});
```

---

## Routing

### Route Guards

```typescript
// Good: Route guards with TypeScript
// router/index.ts
import { createRouter, createWebHistory } from 'vue-router';
import { useUserStore } from '@/stores/user';

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/HomeView.vue'),
    },
    {
      path: '/dashboard',
      name: 'dashboard',
      component: () => import('@/views/DashboardView.vue'),
      meta: { requiresAuth: true },
    },
    {
      path: '/login',
      name: 'login',
      component: () => import('@/views/LoginView.vue'),
      meta: { guest: true },
    },
  ],
});

// Navigation guards
router.beforeEach((to, from, next) => {
  const userStore = useUserStore();

  if (to.meta.requiresAuth && !userStore.isAuthenticated) {
    next({ name: 'login', query: { redirect: to.fullPath } });
  } else if (to.meta.guest && userStore.isAuthenticated) {
    next({ name: 'dashboard' });
  } else {
    next();
  }
});

export default router;
```

### Programmatic Navigation

```vue
<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router';

const router = useRouter();
const route = useRoute();

// Good: Named routes with parameters
const navigateToUser = (userId: string) => {
  router.push({ name: 'user', params: { id: userId } });
};

// Good: Query parameters
const navigateToSearch = (query: string) => {
  router.push({ name: 'search', query: { q: query } });
};

// Good: Navigate back
const goBack = () => {
  router.back();
};

// Good: Replace current route
const navigateToLogin = () => {
  router.replace({ name: 'login' });
};

// Good: Read route parameters
const userId = computed(() => route.params.id as string);
const searchQuery = computed(() => route.query.q as string);
</script>
```

---

## Performance Optimization

### Lazy Loading Components

```vue
<script setup lang="ts">
// Good: Lazy load components
import { defineAsyncComponent } from 'vue';

const HeavyComponent = defineAsyncComponent(() =>
  import('@/components/HeavyComponent.vue')
);

// With loading and error components
const AsyncChart = defineAsyncComponent({
  loader: () => import('@/components/Chart.vue'),
  loadingComponent: LoadingSpinner,
  errorComponent: ErrorDisplay,
  delay: 200,
  timeout: 3000,
});
</script>

<template>
  <Suspense>
    <template #default>
      <HeavyComponent />
    </template>
    <template #fallback>
      <LoadingSpinner />
    </template>
  </Suspense>
</template>
```

### v-memo and v-once

```vue
<template>
  <!-- Good: Use v-memo for expensive renders -->
  <div v-memo="[user.id, user.name]">
    <ExpensiveComponent :user="user" />
  </div>

  <!-- Good: Use v-once for static content -->
  <header v-once>
    <h1>{{ appTitle }}</h1>
    <nav>{{ staticMenu }}</nav>
  </header>

  <!-- Good: v-show vs v-if -->
  <div v-show="isVisible">Toggles visibility, keeps in DOM</div>
  <div v-if="isActive">Adds/removes from DOM</div>
</template>
```

---

## Testing

### Component Testing

```typescript
// Good: Test component with Vitest
import { describe, it, expect } from 'vitest';
import { mount } from '@vue/test-utils';
import UserCard from '@/components/UserCard.vue';

describe('UserCard', () => {
  it('renders user name', () => {
    const wrapper = mount(UserCard, {
      props: {
        user: {
          id: '1',
          name: 'John Doe',
          email: 'john@example.com',
        },
      },
    });

    expect(wrapper.text()).toContain('John Doe');
  });

  it('emits update event when button clicked', async () => {
    const wrapper = mount(UserCard, {
      props: {
        user: {
          id: '1',
          name: 'John Doe',
          email: 'john@example.com',
        },
      },
    });

    await wrapper.find('button.update').trigger('click');
    expect(wrapper.emitted('update')).toBeTruthy();
  });

  it('does not show email when showEmail is false', () => {
    const wrapper = mount(UserCard, {
      props: {
        user: {
          id: '1',
          name: 'John Doe',
          email: 'john@example.com',
        },
        showEmail: false,
      },
    });

    expect(wrapper.find('.email').exists()).toBe(false);
  });
});
```

---

## TypeScript Integration

### Generic Components

```vue
<script setup lang="ts" generic="T extends Item, U extends Item">
interface Item {
  id: string;
}

interface Props {
  items: T[];
  selectedItem?: U;
  onSelect: (item: T) => void;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  selected: [item: T];
}>();
</script>

<template>
  <ul>
    <li
      v-for="item in items"
      :key="item.id"
      @click="emit('selected', item)"
    >
      {{ item }}
    </li>
  </ul>
</template>
```

---

## Anti-Patterns to Avoid

### Don't Mutate Props

```vue
<script setup lang="ts">
// Bad: Directly mutating props
const props = defineProps<{ count: number }>();

const increment = () => {
  props.count++; // Error!
};

// Good: Emit event to parent
const props = defineProps<{ count: number }>();
const emit = defineEmits<{
  (e: 'update:count', value: number): void;
}>();

const increment = () => {
  emit('update:count', props.count + 1);
};
</script>
```

### Don't Over-react

```typescript
// Bad: Everything is reactive
const user = ref({ name: 'John', email: 'john@example.com' });
const userName = computed(() => user.value.name);
const userEmail = computed(() => user.value.email);

// Good: Only make reactive what needs to be
const user = { name: 'John', email: 'john@example.com' };
const userRef = ref(user); // Only make ref if reactivity needed
```

---

## Additional Resources

- [Vue.js Documentation](https://vuejs.org/)
- [Vue Router Documentation](https://router.vuejs.org/)
- [Pinia Documentation](https://pinia.vuejs.org/)
- [VueUse](https://vueuse.org/)
- [Vue Test Utils](https://test-utils.vuejs.org/)
