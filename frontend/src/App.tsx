import './App.css'

function App() {
  return <main className="app"><header><span className="dot" /> R5 Battery Estimator <small>R5 Ultra</small></header><section className="hero"><p>Estado actual</p><strong>—%</strong><span>Esperando una lectura del mouse</span></section><section className="cards"><article><p>Autonomía estimada</p><b>Provisional</b><span>Se ajustará con tu uso real.</span></article><article><p>Última lectura</p><b>Sin datos</b><span>Conectá el receptor 2.4 GHz.</span></article></section><p className="hint">La app nunca mostrará una desconexión como 0%.</p></main>
}

export default App
