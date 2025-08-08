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
  code?: string;
  weight?: number;
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
  importance?: 'high' | 'medium' | 'low';
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

// Generate more realistic and randomized sector data
const generateSectorData = (): Sector[] => {
  const baseSectors = [
    { name: "Technology", code: "XLK", weight: 28.2 },
    { name: "Financials", code: "XLF", weight: 13.1 },
    { name: "Healthcare", code: "XLV", weight: 12.9 },
    { name: "Consumer Disc.", code: "XLY", weight: 10.8 },
    { name: "Communication", code: "XLC", weight: 8.7 },
    { name: "Industrials", code: "XLI", weight: 8.2 },
    { name: "Energy", code: "XLE", weight: 4.2 },
    { name: "Materials", code: "XLB", weight: 2.8 },
    { name: "Utilities", code: "XLU", weight: 2.3 }
  ];

  return baseSectors.map(sector => {
    // Generate more realistic daily changes with some correlation to sector characteristics
    let baseVolatility = 1.2;
    if (sector.code === "XLE") baseVolatility = 2.8; // Energy more volatile
    if (sector.code === "XLK") baseVolatility = 1.8; // Tech moderately volatile
    if (sector.code === "XLU") baseVolatility = 0.6; // Utilities less volatile
    if (sector.code === "XLRE") baseVolatility = 0.8; // REITs less volatile
    
    const change = (Math.random() - 0.5) * baseVolatility * 4;
    return {
      ...sector,
      change: Math.round(change * 100) / 100
    };
  });
};

const sectors = generateSectorData();

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
    const width = 120;
    const height = 30;
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    
    // Use a much more conservative approach - keep all points well within bounds
    const points = data.map((value, index) => {
      const x = (index / (data.length - 1)) * width;
      // Map to a safe zone in the middle 60% of the height, with 20% padding top/bottom
      const normalizedValue = (value - min) / range;
      const y = height * 0.8 - (normalizedValue * height * 0.6) + height * 0.2;
      return { x, y };
    });
    
    // Create smooth curve using simple line segments for cleaner look
    if (points.length < 2) return '';
    
    const pathData = points.map((point, index) => 
      index === 0 ? `M ${point.x},${point.y}` : `L ${point.x},${point.y}`
    ).join(' ');
    
    return pathData;
  };

  const generateSparklineGradient = (ticker: string, isPositive: boolean): string => {
    return `sparkline-gradient-${ticker}-${isPositive ? 'positive' : 'negative'}`;
  };

  // Generate more realistic earnings calendar
  const generateEarningsCalendar = () => {
    const earningsStocks: Array<{ ticker: string; importance: 'high' | 'medium' | 'low' }> = [
      { ticker: "AAPL", importance: "high" },
      { ticker: "MSFT", importance: "high" },
      { ticker: "GOOGL", importance: "high" },
      { ticker: "AMZN", importance: "high" },
      { ticker: "META", importance: "high" },
      { ticker: "TSLA", importance: "high" },
      { ticker: "NVDA", importance: "high" },
      { ticker: "NFLX", importance: "medium" },
      { ticker: "AMD", importance: "medium" },
      { ticker: "CRM", importance: "medium" },
      { ticker: "UBER", importance: "medium" },
      { ticker: "LYFT", importance: "low" },
      { ticker: "SNAP", importance: "low" },
      { ticker: "PINS", importance: "low" },
      { ticker: "SQ", importance: "medium" },
      { ticker: "PYPL", importance: "medium" },
      { ticker: "ZM", importance: "low" },
      { ticker: "SHOP", importance: "medium" },
      { ticker: "SPOT", importance: "low" },
      { ticker: "ROKU", importance: "low" }
    ];

    const calendar: { [key: number]: EarningsItem[] } = {};
    const currentWeekDays = Array.from({ length: 14 }, (_, i) => i + 1);

    currentWeekDays.forEach(day => {
      const dayEarnings: EarningsItem[] = [];
      const numEarnings = Math.floor(Math.random() * 4); // 0-3 earnings per day
      
      for (let i = 0; i < numEarnings; i++) {
        const randomStock = earningsStocks[Math.floor(Math.random() * earningsStocks.length)];
        const time = Math.random() > 0.6 ? "BMO" : "AMC"; // More AMC than BMO
        
        if (!dayEarnings.find(e => e.ticker === randomStock.ticker)) {
          dayEarnings.push({
            ticker: randomStock.ticker,
            time,
            importance: randomStock.importance
          });
        }
      }
      
      calendar[day] = dayEarnings;
    });

    return calendar;
  };

  const [earningsCalendar] = useState(() => generateEarningsCalendar());

  const getEarningsForDay = (day: number): EarningsItem[] => {
    return earningsCalendar[day] || [];
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
              {sectors.map((sector) => {
                const intensity = Math.abs(sector.change);
                const isStrong = intensity > 1.5;
                const isModerate = intensity > 0.8;
                
                return (
                  <div 
                    key={sector.name}
                    className={`${styles.heatmapTile} ${
                      sector.change > 0 ? styles.positive : 
                      sector.change < 0 ? styles.negative : 
                      styles.neutral
                    } ${isStrong ? styles.strong : isModerate ? styles.moderate : styles.weak}`}
                    style={{
                      opacity: 0.6 + (intensity / 4) * 0.4, // Dynamic opacity based on change magnitude
                      fontSize: sector.weight && sector.weight > 10 ? '0.75rem' : '0.7rem'
                    }}
                  >
                    <div className={styles.sectorHeader}>
                      <span className={styles.sectorName}>{sector.name}</span>
                      {sector.code && (
                        <span className={styles.sectorCode}>{sector.code}</span>
                      )}
                    </div>
                    <div className={styles.sectorMetrics}>
                      <span className={styles.sectorChange}>
                        {sector.change > 0 ? '+' : ''}{sector.change.toFixed(2)}%
                      </span>
                      {sector.weight && (
                        <span className={styles.sectorWeight}>
                          {sector.weight.toFixed(1)}%
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}
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
                    <svg width="100%" height="30" viewBox="-4 -4 128 38" preserveAspectRatio="none">
                      <defs>
                        <linearGradient id={generateSparklineGradient(stock.ticker, stock.change > 0)} x1="0%" y1="0%" x2="0%" y2="100%">
                          <stop offset="0%" stopColor={stock.change > 0 ? '#10b981' : '#ef4444'} stopOpacity="0.3"/>
                          <stop offset="100%" stopColor={stock.change > 0 ? '#10b981' : '#ef4444'} stopOpacity="0.1"/>
                        </linearGradient>
                      </defs>
                      
                      {/* Area fill under the curve */}
                      <path 
                        d={`${generateSparklinePath(stock.sparkline)} L 120,30 L 0,30 Z`}
                        fill={`url(#${generateSparklineGradient(stock.ticker, stock.change > 0)})`}
                      />
                      
                      {/* Main price line */}
                      <path 
                        d={generateSparklinePath(stock.sparkline)}
                        fill="none"
                        stroke={stock.change > 0 ? '#10b981' : '#ef4444'}
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
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
              {[...Array(14)].map((_, i) => {
                const day = i + 1;
                const earnings = getEarningsForDay(day);
                const isToday = day === 8; // Different today for variety
                const isFedDay = day === 12; // Fed meeting day
                
                // Get current date info for labeling
                const currentDate = new Date();
                const targetDate = new Date(currentDate);
                targetDate.setDate(currentDate.getDate() + (day - 8)); // Relative to "today"
                const dayOfWeek = targetDate.getDay(); // 0 = Sunday, 6 = Saturday
                const isWeekend = dayOfWeek === 0 || dayOfWeek === 6; // Skip weekends
                
                // Skip rendering weekend days
                if (isWeekend) {
                  return null;
                }
                
                const dayName = targetDate.toLocaleDateString('en-US', { weekday: 'short' });
                const dateNum = targetDate.getDate();
                
                return (
                  <div 
                    key={day}
                    className={`${styles.calendarDay} ${isToday ? styles.today : ''}`}
                  >
                    <div className={styles.dayHeader}>
                      <span className={styles.dayName}>{dayName}</span>
                      <span className={styles.dayNumber}>{dateNum}</span>
                    </div>
                    <div className={styles.earningsContainer}>
                      {isFedDay && (
                        <div className={`${styles.earningsItem} ${styles.fedEarningsItem}`}>
                          <span className={styles.earningsTicker}>FOMC</span>
                          <span className={styles.earningsTime}>14:00</span>
                        </div>
                      )}
                      {earnings.map((item, idx) => (
                        <div 
                          key={idx} 
                          className={`${styles.earningsItem} ${styles[`importance_${item.importance || 'medium'}`]}`}
                          title={`${item.ticker} - ${item.time === 'BMO' ? 'Before Market Open' : 'After Market Close'}`}
                        >
                          <span className={styles.earningsTicker}>{item.ticker}</span>
                          <span className={styles.earningsTime}>{item.time}</span>
                        </div>
                      ))}
                      {earnings.length === 0 && (
                        <span className={styles.noEarnings}>—</span>
                      )}
                    </div>
                  </div>
                );
              }).filter(Boolean)}
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