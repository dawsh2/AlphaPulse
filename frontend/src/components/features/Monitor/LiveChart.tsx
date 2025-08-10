/**
 * Live chart component for market monitoring
 */

import React, { useEffect, useRef } from 'react';
import { createChart, IChartApi, ISeriesApi } from 'lightweight-charts';
import { chartConfig } from '../../../config/charts';
import type { MarketBar } from '../../../types';
import styles from './Monitor.module.css';

interface LiveChartProps {
  data: MarketBar[];
  height?: number;
  showVolume?: boolean;
  onCrosshairMove?: (price: number | null) => void;
}

export const LiveChart: React.FC<LiveChartProps> = ({
  data,
  height = 400,
  showVolume = true,
  onCrosshairMove,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candleSeriesRef = useRef<ISeriesApi<'Candlestick'> | null>(null);
  const volumeSeriesRef = useRef<ISeriesApi<'Histogram'> | null>(null);

  // Initialize chart
  useEffect(() => {
    if (!containerRef.current) return;

    const chart = createChart(containerRef.current, {
      width: containerRef.current.clientWidth,
      height,
      ...chartConfig.lightweightCharts,
    });

    const candleSeries = chart.addCandlestickSeries(chartConfig.candlestick);
    candleSeriesRef.current = candleSeries;

    if (showVolume) {
      const volumeSeries = chart.addHistogramSeries({
        ...chartConfig.volume,
        priceFormat: {
          type: 'volume',
        },
        priceScaleId: 'volume',
      });

      chart.priceScale('volume').applyOptions({
        scaleMargins: {
          top: 0.8,
          bottom: 0,
        },
      });

      volumeSeriesRef.current = volumeSeries;
    }

    chartRef.current = chart;

    // Handle crosshair
    if (onCrosshairMove) {
      chart.subscribeCrosshairMove((param) => {
        if (param.seriesPrices.size > 0) {
          const price = param.seriesPrices.get(candleSeries);
          onCrosshairMove(price as number);
        } else {
          onCrosshairMove(null);
        }
      });
    }

    // Handle resize
    const handleResize = () => {
      if (containerRef.current) {
        chart.applyOptions({ width: containerRef.current.clientWidth });
      }
    };

    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      chart.remove();
    };
  }, [height, showVolume, onCrosshairMove]);

  // Update data
  useEffect(() => {
    if (!candleSeriesRef.current || !data.length) return;

    candleSeriesRef.current.setData(data);

    if (volumeSeriesRef.current) {
      const volumeData = data.map(d => ({
        time: d.time,
        value: d.volume,
        color: d.close >= d.open 
          ? chartConfig.volume.upColor 
          : chartConfig.volume.downColor,
      }));
      volumeSeriesRef.current.setData(volumeData);
    }

    // Auto-fit content
    chartRef.current?.timeScale().fitContent();
  }, [data]);

  return <div ref={containerRef} className={styles.chartContainer} />;
};