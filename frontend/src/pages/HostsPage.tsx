import { useEffect, useState, type FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../api/client'
import type { CreateHostResponse, Host } from '../api/types'

export function HostsPage() {
  const [hosts, setHosts] = useState<Host[]>([])
  const [name, setName] = useState('')
  const [created, setCreated] = useState<CreateHostResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const load = async () => {
    try {
      setHosts(await api.hosts())
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    }
  }

  useEffect(() => {
    void load()
  }, [])

  async function onCreate(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      const res = await api.createHost(name)
      setCreated(res)
      setName('')
      await load()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    } finally {
      setBusy(false)
    }
  }

  async function onDelete(id: string) {
    if (!confirm('Delete this host? Agents using its token will stop authenticating.')) return
    await api.deleteHost(id)
    if (created?.id === id) setCreated(null)
    await load()
  }

  return (
    <section className="page">
      <header className="page-head">
        <h1>Hosts</h1>
        <p>Register a machine, copy the agent token, then docker run the agent.</p>
      </header>

      <form className="inline-form" onSubmit={onCreate}>
        <input
          placeholder="Host name (e.g. prod-box-1)"
          value={name}
          onChange={(e) => setName(e.target.value)}
          required
        />
        <button className="btn primary" type="submit" disabled={busy}>
          Add host
        </button>
      </form>

      {error && <p className="error">{error}</p>}

      {created && (
        <div className="token-box">
          <h2>Agent credentials for {created.name}</h2>
          <p className="warn">Token is shown once. Store it securely.</p>
          <label>
            Agent token
            <code>{created.agent_token}</code>
          </label>
          <label>
            docker run
            <pre>{created.docker_run}</pre>
          </label>
          <button type="button" className="btn ghost" onClick={() => setCreated(null)}>
            Dismiss
          </button>
        </div>
      )}

      <table className="table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Containers</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {hosts.map((h) => (
            <tr key={h.id}>
              <td>
                <Link to={`/hosts/${h.id}`}>{h.name}</Link>
              </td>
              <td>
                <span className={`pill ${h.online ? 'online' : 'offline'}`}>
                  {h.online ? 'online' : 'offline'}
                </span>
              </td>
              <td>
                {h.container_count} ({h.unhealthy_count} bad)
              </td>
              <td className="actions">
                <button type="button" className="btn danger ghost" onClick={() => void onDelete(h.id)}>
                  Delete
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
