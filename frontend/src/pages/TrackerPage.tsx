import { useEffect, useState } from 'react'
import { Navigate } from 'react-router-dom'
import { api } from '../api/client'
import type { ActivityEntry } from '../api/types'
import { useAuth } from '../auth'

function formatWhen(iso: string) {
  try {
    return new Date(iso).toLocaleString()
  } catch {
    return iso
  }
}

export function TrackerPage() {
  const { user } = useAuth()
  const [rows, setRows] = useState<ActivityEntry[]>([])
  const [error, setError] = useState<string | null>(null)
  const isMaster = user?.role === 'master'

  useEffect(() => {
    if (!isMaster) return
    let alive = true
    const load = async () => {
      try {
        const data = await api.tracker(150, 0)
        if (alive) {
          setRows(data)
          setError(null)
        }
      } catch (err) {
        if (alive) setError(err instanceof Error ? err.message : 'failed')
      }
    }
    void load()
    const t = window.setInterval(() => void load(), 8000)
    return () => {
      alive = false
      window.clearInterval(t)
    }
  }, [isMaster])

  if (!isMaster) return <Navigate to="/" replace />

  return (
    <section className="page">
      <header className="page-head">
        <h1>Tracker</h1>
        <p>Activity feed for this account (logins, restarts, logs, invites…).</p>
      </header>

      {error && <p className="error">{error}</p>}

      <table className="table">
        <thead>
          <tr>
            <th>When</th>
            <th>User</th>
            <th>Action</th>
            <th>Resource</th>
            <th>Detail</th>
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr>
              <td colSpan={5} className="muted">
                No activity yet.
              </td>
            </tr>
          ) : (
            rows.map((r) => (
              <tr key={r.id}>
                <td className="mono muted">{formatWhen(r.created_at)}</td>
                <td>{r.username}</td>
                <td className="mono">{r.action}</td>
                <td className="mono muted">
                  {r.resource_type
                    ? `${r.resource_type}${r.resource_id ? `:${r.resource_id.slice(0, 12)}` : ''}`
                    : '—'}
                </td>
                <td className="mono muted">
                  {r.detail ? r.detail.slice(0, 80) : '—'}
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </section>
  )
}
