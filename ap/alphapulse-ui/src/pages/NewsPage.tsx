import React, { useState } from 'react';
import styles from './NewsPage.module.css';

interface Article {
  id: number;
  rank: number;
  title: string;
  url: string;
  source: string;
  author: string;
  tags: { name: string; type: string }[];
  points: number;
  comments: number;
  timeAgo: string;
}

interface Sector {
  name: string;
  change: number;
}

interface Stock {
  ticker: string;
  price: number;
  change: number;
  changePercent: number;
  sparkline: number[];
}

interface EarningsItem {
  ticker: string;
  time: 'BMO' | 'AMC';
}

interface MarketNews {
  time: string;
  headline: string;
}

const sampleArticles: Article[] = [
  {
    id: 1,
    rank: 1,
    title: "Deep Reinforcement Learning for Optimal Order Execution",
    url: "https://arxiv.org/abs/2312.04951",
    source: "arXiv:2312.04951",
    author: "Zhang et al.",
    tags: [{ name: "rl", type: "rl" }, { name: "hft", type: "hft" }],
    points: 287,
    comments: 56,
    timeAgo: "2 hours ago"
  },
  {
    id: 2,
    rank: 2,
    title: "Why Your Sharpe Ratio is Lying: A Statistical Deep Dive",
    url: "#",
    source: "QuantBlog",
    author: "statistician42",
    tags: [{ name: "stats", type: "stats" }, { name: "edu", type: "edu" }],
    points: 234,
    comments: 89,
    timeAgo: "3 hours ago"
  },
  {
    id: 3,
    rank: 3,
    title: "NautilusTrader 1.19.0 Released: Major Performance Improvements",
    url: "#",
    source: "GitHub",
    author: "nautechsystems",
    tags: [{ name: "tool", type: "tool" }, { name: "news", type: "news" }],
    points: 198,
    comments: 34,
    timeAgo: "5 hours ago"
  },
  {
    id: 4,
    rank: 4,
    title: "Options Flow Analysis Using Machine Learning: A Practical Guide",
    url: "#",
    source: "Medium",
    author: "optionstrader99",
    tags: [{ name: "options", type: "options" }, { name: "ml", type: "ml" }],
    points: 156,
    comments: 23,
    timeAgo: "6 hours ago"
  },
  {
    id: 5,
    rank: 5,
    title: "High-Frequency Trading Infrastructure: Building a Sub-Microsecond System",
    url: "#",
    source: "Engineering Blog",
    author: "latency_ninja",
    tags: [{ name: "hft", type: "hft" }, { name: "engineering", type: "quant" }],
    points: 145,
    comments: 67,
    timeAgo: "8 hours ago"
  },
  {
    id: 6,
    rank: 6,
    title: "The Mathematics of Market Making: A Comprehensive Overview",
    url: "#",
    source: "arXiv:2312.05123",
    author: "Liu et al.",
    tags: [{ name: "market-making", type: "quant" }, { name: "math", type: "edu" }],
    points: 134,
    comments: 12,
    timeAgo: "10 hours ago"
  },
  {
    id: 7,
    rank: 7,
    title: "Backtesting Pitfalls: Why 90% of Strategies Fail in Production",
    url: "#",
    source: "QuantStart",
    author: "backtester42",
    tags: [{ name: "backtesting", type: "edu" }, { name: "risk", type: "stats" }],
    points: 128,
    comments: 45,
    timeAgo: "12 hours ago"
  },
  {
    id: 8,
    rank: 8,
    title: "Real-Time Risk Management with Apache Kafka and Flink",
    url: "#",
    source: "Tech Talk",
    author: "stream_processor",
    tags: [{ name: "risk", type: "stats" }, { name: "streaming", type: "tool" }],
    points: 112,
    comments: 8,
    timeAgo: "14 hours ago"
  }
];

const sectors: Sector[] = [
  { name: "Tech", change: 1.23 },
  { name: "Finance", change: -0.45 },
  { name: "Energy", change: 2.10 },
  { name: "Health", change: 0.82 },
  { name: "Consumer", change: -0.12 },
  { name: "Industrial", change: 0.55 },
  { name: "Materials", change: 1.76 },
  { name: "Utilities", change: -0.23 },
  { name: "Real Estate", change: 0.34 }
];

const watchlist: Stock[] = [
  { ticker: "SPY", price: 445.23, change: 2.15, changePercent: 0.48, sparkline: [440, 442, 441, 443, 445, 444, 445] },
  { ticker: "QQQ", price: 373.45, change: 3.78, changePercent: 1.02, sparkline: [368, 370, 369, 372, 374, 373, 373] },
  { ticker: "NVDA", price: 487.23, change: 12.45, changePercent: 2.62, sparkline: [470, 475, 478, 482, 485, 486, 487] },
  { ticker: "AAPL", price: 178.92, change: -1.23, changePercent: -0.68, sparkline: [180, 179, 180, 179, 178, 179, 178] },
  { ticker: "TSLA", price: 238.45, change: 5.67, changePercent: 2.44, sparkline: [230, 232, 235, 234, 237, 238, 238] }
];

const marketNews: MarketNews[] = [
  { time: "14:32", headline: "Fed Minutes: Members See Risks Balanced" },
  { time: "14:15", headline: "Oil Futures Jump on Supply Concerns" },
  { time: "13:45", headline: "Dollar Weakens Against Major Currencies" },
  { time: "13:12", headline: "Tech Stocks Lead Market Higher" }
];

export const NewsPage: React.FC = () => {
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedPdf, setSelectedPdf] = useState<string | null>(null);
  const [showAIChat, setShowAIChat] = useState(false);

  const generateSparklinePath = (data: number[]): string => {
    const width = 60;
    const height = 30;
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    
    const points = data.map((value, index) => {
      const x = (index / (data.length - 1)) * width;
      const y = height - ((value - min) / range) * height;
      return `${x},${y}`;
    });
    
    return `M ${points.join(' L ')}`;
  };

  const getEarningsForDay = (day: number): EarningsItem[] => {
    const earnings: { [key: number]: EarningsItem[] } = {
      3: [{ ticker: "MSFT", time: "AMC" }, { ticker: "META", time: "AMC" }],
      5: [{ ticker: "AMZN", time: "AMC" }],
      8: [{ ticker: "GOOGL", time: "AMC" }, { ticker: "AMD", time: "AMC" }],
      10: [{ ticker: "NFLX", time: "BMO" }],
    };
    return earnings[day] || [];
  };

  return (
    <div className={styles.newsContainer}>
      <div className={styles.contentGrid}>
        {/* Left Column - Link Submissions */}
        <div className={styles.submissionsColumn}>
          <div className={styles.articleList}>
            {sampleArticles.map((article) => (
              <article key={article.id} className={styles.article}>
                <span className={styles.articleRank}>{article.rank}.</span>
                <div className={styles.articleContent}>
                  <a 
                    href={article.url} 
                    className={styles.articleHeadline}
                    onClick={(e) => {
                      if (article.url.includes('arxiv')) {
                        e.preventDefault();
                        setSelectedPdf(article.url);
                      }
                    }}
                  >
                    {article.title}
                  </a>
                  <div className={styles.articleSource}>
                    <span>{article.source}</span>
                    <span className={styles.dot}></span>
                    <span>{article.author}</span>
                    <div className={styles.articleTags}>
                      {article.tags.map((tag, idx) => (
                        <span key={idx} className={`${styles.tag} ${styles[tag.type]}`}>
                          {tag.name}
                        </span>
                      ))}
                    </div>
                  </div>
                  <div className={styles.articleMeta}>
                    <span>{article.points} points</span>
                    <span>•</span>
                    <a href="#" className={styles.commentsLink}>
                      {article.comments} comments
                    </a>
                    <span>•</span>
                    <span>{article.timeAgo}</span>
                  </div>
                </div>
              </article>
            ))}
          </div>
          
          <div className={styles.pageNavigation}>
            <div className={styles.pageTurner}>
              <button 
                className={styles.pageBtn}
                onClick={() => setCurrentPage(Math.max(1, currentPage - 1))}
                disabled={currentPage === 1}
              >
                ← prev
              </button>
              <span className={styles.pageCurrent}>{currentPage}</span>
              <button 
                className={styles.pageBtn}
                onClick={() => setCurrentPage(currentPage + 1)}
              >
                next →
              </button>
            </div>
            <a href="#" className={styles.submitLink}>
              submit →
            </a>
          </div>
          
          {/* Search Bar */}
          <div className={styles.searchBar}>
            <input 
              type="text" 
              className={styles.searchInput}
              placeholder="Search AlphaPulse..."
            />
          </div>
        </div>

        {/* Right Column - Dashboard Grid */}
        <div className={styles.dashboardGrid}>
          {/* Sector Heatmap */}
          <div className={styles.dashboardCard}>
            <h3 className={styles.cardTitle}>Sector Performance</h3>
            <div className={styles.heatmapGrid}>
              {sectors.map((sector) => (
                <div 
                  key={sector.name}
                  className={`${styles.heatmapTile} ${
                    sector.change > 0 ? styles.positive : 
                    sector.change < 0 ? styles.negative : 
                    styles.neutral
                  }`}
                >
                  <span className={styles.sectorName}>{sector.name}</span>
                  <span className={styles.sectorChange}>
                    {sector.change > 0 ? '+' : ''}{sector.change.toFixed(2)}%
                  </span>
                </div>
              ))}
            </div>
          </div>

          {/* Watchlist */}
          <div className={styles.dashboardCard}>
            <h3 className={styles.cardTitle}>Watchlist</h3>
            <div className={styles.watchlistItems}>
              {watchlist.map((stock) => (
                <div key={stock.ticker} className={styles.watchlistItem}>
                  <div className={styles.tickerInfo}>
                    <span className={styles.ticker}>{stock.ticker}</span>
                    <span className={`${styles.change} ${stock.change > 0 ? styles.positive : styles.negative}`}>
                      {stock.change > 0 ? '+' : ''}{stock.changePercent.toFixed(2)}%
                    </span>
                  </div>
                  <div className={styles.sparkline}>
                    <svg width="60" height="30" viewBox="0 0 60 30">
                      <path 
                        d={generateSparklinePath(stock.sparkline)}
                        fill="none"
                        stroke={stock.change > 0 ? '#10b981' : '#ef4444'}
                        strokeWidth="2"
                      />
                    </svg>
                  </div>
                  <div className={styles.priceInfo}>
                    <span className={styles.price}>${stock.price.toFixed(2)}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Earnings Calendar */}
          <div className={styles.dashboardCard}>
            <h3 className={styles.cardTitle}>Earnings Calendar</h3>
            <div className={styles.calendarGrid}>
              {[...Array(10)].map((_, i) => {
                const day = i + 1;
                const earnings = getEarningsForDay(day);
                const isToday = day === 5;
                const isFedDay = day === 7;
                
                return (
                  <div 
                    key={day}
                    className={`${styles.calendarDay} ${isToday ? styles.today : ''}`}
                  >
                    <span className={styles.dayNumber}>{day}</span>
                    {isFedDay && <span className={styles.fedIndicator}>FED</span>}
                    {earnings.map((item, idx) => (
                      <span key={idx} className={styles.earningsDot}>
                        {item.ticker}
                      </span>
                    ))}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Market News */}
          <div className={styles.dashboardCard}>
            <h3 className={styles.cardTitle}>Market Headlines</h3>
            <div className={styles.newsItems}>
              {marketNews.map((item, idx) => (
                <div key={idx} className={styles.newsItem}>
                  <span className={styles.newsTime}>{item.time}</span>
                  <span className={styles.newsHeadline}>{item.headline}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* AI Chat Terminal - Hidden by default */}
      {showAIChat && (
        <div className={styles.aiChatTerminal}>
          <div className={styles.terminalHeader}>
            <h3 className={styles.terminalTitle}>ALPHA RESEARCH TERMINAL</h3>
            <button 
              className={styles.terminalClose}
              onClick={() => setShowAIChat(false)}
            >
              ×
            </button>
          </div>
          <div className={styles.terminalContent}>
            <div className={styles.aiMessage + ' ' + styles.user}>
              Show me the latest papers on transformer models in finance
            </div>
            <div className={styles.aiMessage + ' ' + styles.assistant}>
              Here are the most recent papers on transformer models applied to finance:

              1. "FinBERT: Financial Sentiment Analysis with Pre-trained Language Models" - Analyzes financial text using BERT architecture
              
              2. "Temporal Fusion Transformers for Interpretable Multi-horizon Time Series Forecasting" - Google's approach to financial time series
              
              3. "StockFormer: Learning Hybrid Trading Patterns with Transformers" - Novel architecture combining technical and fundamental analysis
            </div>
          </div>
          <div className={styles.aiChatInputWrapper}>
            <span className={styles.aiChatPrompt}>{'>'}</span>
            <input 
              type="text" 
              className={styles.aiChatInput}
              placeholder="Ask about papers, strategies, or market analysis..."
            />
          </div>
        </div>
      )}

      {/* PDF Reader Overlay */}
      {selectedPdf && (
        <div className={styles.pdfOverlay} onClick={() => setSelectedPdf(null)}>
          <div className={styles.pdfReader} onClick={(e) => e.stopPropagation()}>
            <div className={styles.pdfHeader}>
              <h3 className={styles.pdfTitle}>PDF Reader</h3>
              <button 
                className={styles.pdfClose}
                onClick={() => setSelectedPdf(null)}
              >
                ×
              </button>
            </div>
            <div className={styles.pdfContent}>
              <p>PDF viewer for: {selectedPdf}</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};