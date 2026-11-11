import React from 'react';
import { ReplayWorkspace } from '../components/replay/ReplayWorkspace';

/** Route-level composition surface. Replay orchestration lives below this boundary. */
export const ReplayPage: React.FC = () => <ReplayWorkspace />;
