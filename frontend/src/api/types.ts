export type User = {
  id: string
  username: string
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
}

export type CreateHostResponse = {
  id: string
  name: string
  agent_token: string
  docker_run: string
}
