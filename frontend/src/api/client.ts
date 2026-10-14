async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    credentials: 'include',
    headers: {
      'Content-Type': 'application/json',
      ...(init?.headers ?? {}),
    },
    ...init,
  })

  if (!res.ok) {
    let message = res.statusText
    try {
      const body = (await res.json()) as { error?: string }
      if (body.error) message = body.error
    } catch {
      /* ignore */
    }
    throw new Error(message)
  }

  if (res.status === 204) return undefined as T
  return (await res.json()) as T
}

export const api = {
  me: () => request<import('./types').User>('/api/auth/me'),
  register: (username: string, password: string) =>
    request<import('./types').User>('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  login: (username: string, password: string) =>
    request<import('./types').User>('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    }),
  logout: () =>
    request<{ ok: boolean }>('/api/auth/logout', { method: 'POST' }),
  hosts: () => request<import('./types').Host[]>('/api/hosts'),
  createHost: (name: string) =>
    request<import('./types').CreateHostResponse>('/api/hosts', {
      method: 'POST',
      body: JSON.stringify({ name }),
    }),
  deleteHost: (id: string) =>
    request<{ ok: boolean }>(`/api/hosts/${id}`, { method: 'DELETE' }),
  containers: (hostId: string) =>
    request<import('./types').Container[]>(`/api/hosts/${hostId}/containers`),
  logs: (hostId: string, cid: string, tail = 200) =>
    request<{ logs: string }>(
      `/api/hosts/${hostId}/containers/${cid}/logs?tail=${tail}`,
    ),
  restart: (hostId: string, cid: string) =>
    request<{ ok: boolean }>(
      `/api/hosts/${hostId}/containers/${cid}/restart`,
      { method: 'POST' },
    ),
}
