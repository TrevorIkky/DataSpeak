"use client"

import { PolarAngleAxis, PolarGrid, Radar, RadarChart } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from "@/components/ui/chart";

interface RadarChartRendererProps {
  data: Record<string, any>[];
  xAxis: string;
  yAxis: string[];
}

export function RadarChartRenderer({ data, xAxis, yAxis }: RadarChartRendererProps) {
  // Generate chart config from yAxis keys
  const chartConfig: ChartConfig = yAxis.reduce((config, key, index) => {
    config[key] = {
      label: key.charAt(0).toUpperCase() + key.slice(1),
      color: `var(--chart-${(index % 5) + 1})`,
    };
    return config;
  }, {} as ChartConfig);

  return (
    <ChartContainer config={chartConfig} className="h-full w-full">
      <RadarChart data={data}>
        <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
        <PolarAngleAxis dataKey={xAxis} />
        <PolarGrid />
        {yAxis.length > 1 && <ChartLegend content={<ChartLegendContent />} />}
        {yAxis.map((key) => (
          <Radar
            key={key}
            dataKey={key}
            fill={`var(--color-${key})`}
            fillOpacity={0.6}
            stroke={`var(--color-${key})`}
            strokeWidth={2}
          />
        ))}
      </RadarChart>
    </ChartContainer>
  );
}
