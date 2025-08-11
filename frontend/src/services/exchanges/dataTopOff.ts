/**
 * Service to "top off" stored data with recent data before starting live feed
 * This ensures continuous time series without gaps
 */

import type { MarketData, ExchangeService } from './types';
import { dataStorage } from '../data';

interface TopOffResult {
  success: boolean;
  candlesAdded: number;
  gapMinutes: number;
  message: string;
}

export async function topOffStoredData(
  symbol: string, 
  exchange: string,
  service: ExchangeService,
  storedData: MarketData[]
): Promise<{ data: MarketData[], result: TopOffResult }> {
  if (storedData.length === 0) {
    return {
      data: storedData,
      result: {
        success: false,
        candlesAdded: 0,
        gapMinutes: 0,
        message: 'No stored data to top off'
      }
    };
  }

  const lastCandle = storedData[storedData.length - 1];
  const now = Math.floor(Date.now() / 1000);
  const gapMinutes = Math.floor((now - lastCandle.time) / 60);
  
  console.log(`[${exchange}] Last stored candle: ${new Date(lastCandle.time * 1000).toISOString()}`);
  console.log(`[${exchange}] Gap to fill: ${gapMinutes} minutes`);

  // If gap is less than 2 minutes, data is fresh enough
  // Changed from 5 to 2 to ensure we don't skip small gaps
  if (gapMinutes < 2) {
    return {
      data: storedData,
      result: {
        success: true,
        candlesAdded: 0,
        gapMinutes,
        message: `Data is fresh (${gapMinutes} min old)`
      }
    };
  }

  // If gap is huge (> 1 day), we might want to re-fetch everything
  if (gapMinutes > 1440) {
    console.warn(`[${exchange}] Gap is too large (${gapMinutes} min), consider re-fetching all data`);
    // For now, try to fetch the last day worth of data
  }

  try {
    let updatedData = [...storedData];
    let totalCandlesAdded = 0;
    
    // Always fetch enough data to cover the gap plus a small buffer
    // Add 5 minutes buffer to ensure we get all candles
    const minutesToFetch = Math.min(gapMinutes + 5, 1440); // Max 1 day
    
    console.log(`[${exchange}] Fetching ${minutesToFetch} minutes to fill gap of ${gapMinutes} minutes...`);
    const recentData = await service.fetchHistoricalData(symbol, minutesToFetch);
    
    if (recentData.length === 0) {
      console.warn(`[${exchange}] No data returned for gap fill`);
      return {
        data: storedData,
        result: {
          success: false,
          candlesAdded: 0,
          gapMinutes,
          message: `Failed to fetch data for gap (${gapMinutes} min)`
        }
      };
    }
    
    console.log(`[${exchange}] Fetched ${recentData.length} candles from API`);
    
    // Sort the fetched data by time
    recentData.sort((a, b) => a.time - b.time);
    
    // Find the last stored time
    const lastStoredTime = updatedData[updatedData.length - 1].time;
    
    // Filter to only add candles newer than what we have
    const newCandles = recentData.filter(c => c.time > lastStoredTime);
    
    console.log(`[${exchange}] Found ${newCandles.length} new candles to add`);
    
    if (newCandles.length === 0) {
      console.warn(`[${exchange}] No new candles in fetched data - data might be up to date`);
      return {
        data: storedData,
        result: {
          success: true,
          candlesAdded: 0,
          gapMinutes,
          message: `No new data available (gap: ${gapMinutes} min)`
        }
      };
    }
    
    // Add new candles
    updatedData = [...updatedData, ...newCandles];
    totalCandlesAdded = newCandles.length;
    
    // Sort by time to ensure correct order
    updatedData.sort((a, b) => a.time - b.time);
    
    // Store the new candles in IndexedDB
    const storedCandles = newCandles.map(candle => ({
      symbol,
      exchange,
      interval: '1m' as const,
      timestamp: candle.time,
      open: candle.open,
      high: candle.high,
      low: candle.low,
      close: candle.close,
      volume: candle.volume,
      metadata: {
        fetchedAt: Date.now(),
        source: 'exchange-topoff'
      }
    }));
    
    await dataStorage.saveCandles(storedCandles);
    
    // Calculate actual remaining gap after adding new candles
    const newLastCandle = updatedData[updatedData.length - 1];
    const remainingGap = Math.floor((now - newLastCandle.time) / 60);
    
    console.log(`[${exchange}] Added ${totalCandlesAdded} candles, ${remainingGap} min gap remains`);
    
    // Remove duplicates (in case of overlapping data)
    const uniqueData = updatedData.filter((candle, index, array) => {
      if (index === 0) return true;
      return candle.time !== array[index - 1].time;
    });
    
    return {
      data: uniqueData,
      result: {
        success: true,
        candlesAdded: totalCandlesAdded,
        gapMinutes: remainingGap,
        message: `Added ${totalCandlesAdded} candles, ${remainingGap} min gap remains`
      }
    };
    
  } catch (error) {
    console.error(`[${exchange}] Failed to top off data:`, error);
    return {
      data: storedData,
      result: {
        success: false,
        candlesAdded: 0,
        gapMinutes,
        message: `Error: ${error instanceof Error ? error.message : 'Unknown error'}`
      }
    };
  }
}