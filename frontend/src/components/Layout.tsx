import { Link, NavLink, Outlet } from 'react-router-dom'
import { useAuth } from '../auth'

export function Layout() {
  const { user, logout } = useAuth()
  const isMaster = user?.role === 'master'

  return (
    <div className="shell">
      <header className="topbar">
        <Link to="/" className="brand">
          Monitor Buddy
        </Link>
        <nav>
          <NavLink to="/" end>
            Dashboard
          </NavLink>
          <NavLink to="/hosts">Hosts</NavLink>
          {isMaster && <NavLink to="/users">Users</NavLink>}
          {isMaster && <NavLink to="/tracker">Tracker</NavLink>}
        </nav>
        <div className="topbar-right">
          <span className="user">
            {user?.username}
            {isMaster ? ' · master' : ''}
          </span>
          <button type="button" className="btn ghost" onClick={() => void logout()}>
            Logout
          </button>
        </div>
      </header>
      <main className="main">
        <Outlet />
      </main>
    </div>
  )
}
