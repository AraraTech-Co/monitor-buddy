export type UserRole = 'master' | 'member'

export type User = {
  id: string
  username: string
  role: UserRole
}

export type Host = {
  id: string
  name: string
  online: boolean
  last_seen_at: string | null
  container_count: number
  unhealthy_count: number
  created_at: string
}

export type Container = {
  id: string
  name: string
  image: string
  status: string
  state: string
  health: string | null
  has_healthcheck: boolean
  overwatch: boolean
}

export type CreateHostResponse = {
  id: string
  name: string
  agent_token: string
  docker_run: string
}

export type ActivityEntry = {
  id: string
  user_id: string
  username: string
  action: string
  resource_type: string | null
  resource_id: string | null
  detail: string | null
  created_at: string
}
