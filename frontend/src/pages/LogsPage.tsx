import { useEffect, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { api } from '../api/client'

export function LogsPage() {
  const { id = '', cid = '' } = useParams()
  const [logs, setLogs] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [tail, setTail] = useState(200)
  const [loading, setLoading] = useState(true)

  const load = async () => {
    setLoading(true)
    try {
      const res = await api.logs(id, cid, tail)
      setLogs(res.logs)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void load()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, cid, tail])

  return (
    <section className="page">
      <header className="page-head">
        <p className="crumb">
          <Link to={`/hosts/${id}`}>Back to host</Link>
        </p>
        <h1>Logs</h1>
        <p className="mono muted">{cid}</p>
      </header>

      <div className="inline-form">
        <label className="inline">
          Tail
          <input
            type="number"
            min={1}
            max={5000}
            value={tail}
            onChange={(e) => setTail(Number(e.target.value) || 200)}
          />
        </label>
        <button type="button" className="btn ghost" onClick={() => void load()}>
          Refresh
        </button>
      </div>

      {error && <p className="error">{error}</p>}
      <pre className="log-view">{loading ? 'Loading…' : logs || '(empty)'}</pre>
    </section>
  )
}
