import { useEffect, useState, type FormEvent } from 'react'
import { Navigate } from 'react-router-dom'
import { api } from '../api/client'
import type { User } from '../api/types'
import { useAuth } from '../auth'

export function UsersPage() {
  const { user } = useAuth()
  const [users, setUsers] = useState<User[]>([])
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const isMaster = user?.role === 'master'

  const load = async () => {
    try {
      setUsers(await api.listUsers())
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    }
  }

  useEffect(() => {
    if (!isMaster) return
    void load()
  }, [isMaster])

  if (!isMaster) return <Navigate to="/" replace />

  async function onCreate(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    try {
      await api.createUser(username, password)
      setUsername('')
      setPassword('')
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    } finally {
      setBusy(false)
    }
  }

  async function onDelete(id: string) {
    if (!confirm('Remove this user?')) return
    try {
      await api.deleteUser(id)
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    }
  }

  return (
    <section className="page">
      <header className="page-head">
        <h1>Users</h1>
        <p>Invite members to this account. Only master can manage users.</p>
      </header>

      <form className="inline-form" onSubmit={onCreate}>
        <input
          placeholder="Username"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          required
          minLength={3}
        />
        <input
          type="password"
          placeholder="Password (min 8)"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          minLength={8}
        />
        <button className="btn primary" type="submit" disabled={busy}>
          Add member
        </button>
      </form>

      {error && <p className="error">{error}</p>}

      <table className="table">
        <thead>
          <tr>
            <th>Username</th>
            <th>Role</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {users.map((u) => (
            <tr key={u.id}>
              <td>{u.username}</td>
              <td>
                <span className={`pill ${u.role === 'master' ? 'online' : 'neutral'}`}>
                  {u.role}
                </span>
              </td>
              <td className="actions">
                {u.id !== user?.id && (
                  <button
                    type="button"
                    className="btn danger ghost"
                    onClick={() => void onDelete(u.id)}
                  >
                    Remove
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
