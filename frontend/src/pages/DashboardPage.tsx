import { useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { api } from '../api/client'
import type { Host } from '../api/types'

export function DashboardPage() {
  const [hosts, setHosts] = useState<Host[]>([])
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    const load = async () => {
      try {
        const data = await api.hosts()
        if (alive) {
          setHosts(data)
          setError(null)
        }
      } catch (err) {
        if (alive) setError(err instanceof Error ? err.message : 'failed')
      }
    }
    void load()
    const id = window.setInterval(() => void load(), 5000)
    return () => {
      alive = false
      window.clearInterval(id)
    }
  }, [])

  const chartData = useMemo(
    () =>
      hosts.map((h) => ({
        name: h.name,
        containers: h.container_count,
        unhealthy: h.unhealthy_count,
      })),
    [hosts],
  )

  const online = hosts.filter((h) => h.online).length
  const unhealthy = hosts.reduce((n, h) => n + h.unhealthy_count, 0)

  return (
    <section className="page">
      <header className="page-head">
        <h1>Dashboard</h1>
        <p>Live snapshot of registered hosts and container health.</p>
      </header>

      {error && <p className="error">{error}</p>}

      <div className="stat-row">
        <div className="stat">
          <span className="stat-label">Hosts</span>
          <strong>{hosts.length}</strong>
        </div>
        <div className="stat">
          <span className="stat-label">Online</span>
          <strong className="ok">{online}</strong>
        </div>
        <div className="stat">
          <span className="stat-label">Unhealthy / stopped</span>
          <strong className={unhealthy ? 'bad' : 'ok'}>{unhealthy}</strong>
        </div>
      </div>

      <div className="chart-block">
        <h2>Containers by host</h2>
        {chartData.length === 0 ? (
          <p className="muted">No hosts yet. Register one under Hosts.</p>
        ) : (
          <div className="chart-wrap">
            <ResponsiveContainer width="100%" height={280}>
              <BarChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.08)" />
                <XAxis dataKey="name" stroke="#9aa7b5" />
                <YAxis allowDecimals={false} stroke="#9aa7b5" />
                <Tooltip
                  contentStyle={{
                    background: '#14212d',
                    border: '1px solid #2a3c4d',
                    borderRadius: 8,
                  }}
                />
                <Bar dataKey="containers" fill="#3dbb8a" name="Containers" radius={[4, 4, 0, 0]} />
                <Bar dataKey="unhealthy" fill="#e07a5f" name="Unhealthy" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>

      <div className="host-grid">
        {hosts.map((h) => (
          <Link key={h.id} to={`/hosts/${h.id}`} className="host-tile">
            <div className="host-tile-top">
              <strong>{h.name}</strong>
              <span className={`pill ${h.online ? 'online' : 'offline'}`}>
                {h.online ? 'online' : 'offline'}
              </span>
            </div>
            <p>
              {h.container_count} containers · {h.unhealthy_count} unhealthy
            </p>
          </Link>
        ))}
      </div>
    </section>
  )
}
