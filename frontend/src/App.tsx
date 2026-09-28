import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

type Probe =
  | { status: 'ok'; reading: { percent: number; charging: boolean } }
  | { status: 'device_unavailable' | 'busy' | 'timeout' | 'malformed_report' | 'hid_error' | 'worker_stopped'; message: string }

function App() {
  const [probe, setProbe] = useState<Probe | null>(null)
  const refresh = useCallback(() => invoke<Probe>('refresh_battery').then(setProbe).catch((error) => setProbe({ status: 'hid_error', message: String(error) })), [])
  useEffect(() => {
    const readCached = () => void invoke<Probe>('current_battery').then(setProbe).catch((error) => setProbe({ status: 'hid_error', message: String(error) }))
    readCached()
    const timer = window.setInterval(readCached, 30_000)
    return () => window.clearInterval(timer)
  }, [])
  const reading = probe?.status === 'ok' ? probe.reading : null
  const estimate = reading ? Math.round(reading.percent * 2) : null
  return <main className="app"><header><span className="dot" /> R5 Battery Estimator <small>R5 Ultra</small></header><section className="hero"><p>Estado actual</p><strong>{reading ? `${reading.percent}%` : '—%'}</strong><span>{reading ? (reading.charging ? 'Cargando' : 'No cargando') : (probe?.status === 'device_unavailable' ? 'Receptor no conectado' : 'Esperando una lectura del mouse')}</span><button onClick={() => void refresh()}>Actualizar ahora</button></section><section className="cards"><article><p>Autonomía estimada</p><b>{estimate === null ? '—' : `${estimate} h`}</b><span>Provisional: calculada sobre hasta 200 h.</span></article><article><p>Última lectura</p><b>{reading ? 'Disponible' : 'Sin datos'}</b><span>El monitor se actualiza cada 30 segundos.</span></article></section><p className="hint">La app nunca mostrará una desconexión como 0%.</p></main>
}

export default App
