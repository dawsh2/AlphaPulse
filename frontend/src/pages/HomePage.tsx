import React from 'react';
import styles from './HomePage.module.css';

export const HomePage: React.FC = () => {
  return (
    <div className={styles.container}>
      <section className={styles.hero}>
        <div className={styles.heroContent}>
          <h1 className={styles.title}>
            Event-Driven Quantitative Trading
          </h1>
          <p className={styles.subtitle}>
            Build, test, and deploy sophisticated trading strategies with AlphaPulse's 
            professional-grade platform powered by NautilusTrader.
          </p>
          <div className={styles.ctaButtons}>
            <button className={styles.ctaPrimary}>Get Started</button>
            <button className={styles.ctaSecondary}>View Documentation</button>
          </div>
        </div>
      </section>

      <section className={styles.features}>
        <div className={styles.featuresGrid}>
          <div className={styles.featureCard}>
            <h3>Strategy Development</h3>
            <p>Build complex trading strategies with our intuitive visual builder or code editor.</p>
          </div>
          <div className={styles.featureCard}>
            <h3>Backtesting Engine</h3>
            <p>Test your strategies against historical data with millisecond precision.</p>
          </div>
          <div className={styles.featureCard}>
            <h3>Live Trading</h3>
            <p>Deploy strategies to live markets with built-in risk management.</p>
          </div>
          <div className={styles.featureCard}>
            <h3>Real-time Analytics</h3>
            <p>Monitor performance with comprehensive dashboards and alerts.</p>
          </div>
        </div>
      </section>
    </div>
  );
};