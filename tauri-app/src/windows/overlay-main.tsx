import React from 'react';
import ReactDOM from 'react-dom/client';
import { OverlayWindow } from './Overlay';
import '../styles/overlay.css';

ReactDOM.createRoot(document.getElementById('overlay-root') as HTMLElement).render(
  <OverlayWindow />,
);
