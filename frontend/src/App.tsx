import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

type Probe = { status: 'ok', reading: { percent: number, charging: boolean } } | { status: string, message: string }

function App() {
  const [probe, setProbe] = useState<Probe | null>(null)
  useEffect(() => { invoke<Probe>('current_battery').then(setProbe).catch((error) => setProbe({ status: 'hid_error', message: String(error) })) }, [])
  const reading = probe?.status === 'ok' ? probe.reading : null
  return <main className="app"><header><span className="dot" /> R5 Battery Estimator <small>R5 Ultra</small></header><section className="hero"><p>Estado actual</p><strong>{reading ? `${reading.percent}%` : '—%'}</strong><span>{reading ? (reading.charging ? 'Cargando' : 'No cargando') : (probe?.status === 'device_unavailable' ? 'Receptor no conectado' : 'Esperando una lectura del mouse')}</span></section><section className="cards"><article><p>Autonomía estimada</p><b>Provisional</b><span>Se ajustará con tu uso real.</span></article><article><p>Última lectura</p><b>{reading ? 'Ahora' : 'Sin datos'}</b><span>Una desconexión nunca se muestra como 0%.</span></article></section><p className="hint">La app nunca mostrará una desconexión como 0%.</p></main>
}

export default App
