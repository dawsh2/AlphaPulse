import React from 'react';
// Keep old component for now, can switch when ready
import MonitorPageComponent from '../components/MonitorPage';
// New refactored version
// import { MonitorContainer } from '../components/features/Monitor/MonitorContainer';

const MonitorPage: React.FC = () => {
  // Option 1: Use old component (current)
  return <MonitorPageComponent />;
  
  // Option 2: Use new refactored component
  // return (
  //   <MonitorContainer
  //     symbol="BTC/USD"
  //     exchange="COINBASE"
  //     timeframe="1m"
  //   />
  // );
};

export default MonitorPage;