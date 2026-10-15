import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'
import type { Container, Host } from '../api/types'

export function HostDetailPage() {
  const { id = '' } = useParams()
  const [host, setHost] = useState<Host | null>(null)
  const [containers, setContainers] = useState<Container[]>([])
  const [error, setError] = useState<string | null>(null)
  const [restarting, setRestarting] = useState<string | null>(null)
  const [toggling, setToggling] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    const load = async () => {
      try {
        const [hosts, list] = await Promise.all([api.hosts(), api.containers(id)])
        if (!alive) return
        setHost(hosts.find((h) => h.id === id) ?? null)
        setContainers(list)
        setError(null)
      } catch (err) {
        if (alive) setError(err instanceof Error ? err.message : 'failed')
      }
    }
    void load()
    const t = window.setInterval(() => void load(), 5000)
    return () => {
      alive = false
      window.clearInterval(t)
    }
  }, [id])

  async function restart(cid: string) {
    setRestarting(cid)
    try {
      await api.restart(id, cid)
      setContainers(await api.containers(id))
    } catch (err) {
      setError(err instanceof Error ? err.message : 'restart failed')
    } finally {
      setRestarting(null)
    }
  }

  async function toggleOverwatch(c: Container, enabled: boolean) {
    if (!c.has_healthcheck) return
    setToggling(c.id)
    try {
      await api.setOverwatch(id, c.id, enabled)
      setContainers(await api.containers(id))
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'overwatch update failed')
    } finally {
      setToggling(null)
    }
  }

  return (
    <section className="page">
      <header className="page-head">
        <p className="crumb">
          <Link to="/hosts">Hosts</Link> / {host?.name ?? id}
        </p>
        <h1>{host?.name ?? 'Host'}</h1>
        <p>
          {host ? (
            <>
              <span className={`pill ${host.online ? 'online' : 'offline'}`}>
                {host.online ? 'online' : 'offline'}
              </span>{' '}
              · {containers.length} containers
            </>
          ) : (
            'Loading…'
          )}
        </p>
      </header>

      {error && <p className="error">{error}</p>}

      <table className="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Image</th>
            <th>State</th>
            <th>Health</th>
            <th>Overwatch</th>
            <th>Status</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {containers.map((c) => (
            <tr key={c.id}>
              <td className="mono">{c.name}</td>
              <td className="mono muted">{c.image}</td>
              <td>{c.state}</td>
              <td>
                <span
                  className={`pill ${
                    c.health === 'healthy'
                      ? 'online'
                      : c.health === 'unhealthy'
                        ? 'offline'
                        : 'neutral'
                  }`}
                >
                  {c.health ?? 'n/a'}
                </span>
              </td>
              <td>
                <label
                  className={`switch ${!c.has_healthcheck ? 'disabled' : ''} ${c.overwatch ? 'on' : ''}`}
                  title={
                    c.has_healthcheck
                      ? 'Auto-restart when unhealthy'
                      : 'Requires a Docker healthcheck'
                  }
                >
                  <input
                    type="checkbox"
                    checked={c.overwatch}
                    disabled={!c.has_healthcheck || toggling === c.id}
                    onChange={(e) => void toggleOverwatch(c, e.target.checked)}
                  />
                  <span className="switch-track" />
                  <span className="switch-label">
                    {c.has_healthcheck ? (c.overwatch ? 'On' : 'Off') : 'N/A'}
                  </span>
                </label>
              </td>
              <td className="muted">{c.status}</td>
              <td className="actions">
                <Link className="btn ghost" to={`/hosts/${id}/containers/${c.id}/logs`}>
                  Logs
                </Link>
                <button
                  type="button"
                  className="btn primary"
                  disabled={restarting === c.id}
                  onClick={() => void restart(c.id)}
                >
                  {restarting === c.id ? '…' : 'Restart'}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
