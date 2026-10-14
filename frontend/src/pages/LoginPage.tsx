import { useState, type FormEvent } from 'react'
import { Navigate } from 'react-router-dom'
import { useAuth } from '../auth'

export function LoginPage() {
  const { user, loading, login, register } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  if (!loading && user) return <Navigate to="/" replace />

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setBusy(true)
    setError(null)
    try {
      if (mode === 'login') await login(username, password)
      else await register(username, password)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'failed')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="login-screen">
      <div className="login-panel">
        <p className="eyebrow">Docker fleet</p>
        <h1 className="brand-hero">Monitor Buddy</h1>
        <p className="lede">
          Watch health, tail logs, and restart containers across your machines.
        </p>

        <form onSubmit={onSubmit} className="stack">
          <label>
            Username
            <input
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              autoComplete="username"
              required
              minLength={3}
            />
          </label>
          <label>
            Password
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete={mode === 'login' ? 'current-password' : 'new-password'}
              required
              minLength={8}
            />
          </label>
          {error && <p className="error">{error}</p>}
          <button className="btn primary" disabled={busy} type="submit">
            {busy ? '…' : mode === 'login' ? 'Sign in' : 'Create first user'}
          </button>
        </form>

        <button
          type="button"
          className="linkish"
          onClick={() => setMode(mode === 'login' ? 'register' : 'login')}
        >
          {mode === 'login'
            ? 'First run? Register bootstrap user'
            : 'Already have an account? Sign in'}
        </button>
      </div>
      <div className="login-visual" aria-hidden />
    </div>
  )
}
