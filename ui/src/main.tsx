import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { registerAllCustomIndicators } from './chart/indicators/registerAll'
import { registerAllCustomOverlays } from './chart/overlays/registerAll'
import App from './App'
import './index.css'

// Strictly register all 37+ overlays and 5 Sumi Core custom indicators BEFORE any chart init()
registerAllCustomOverlays()
registerAllCustomIndicators()

const rootEl = document.getElementById('root')
if (rootEl) {
  createRoot(rootEl).render(
    <StrictMode>
      <App />
    </StrictMode>,
  )
}
