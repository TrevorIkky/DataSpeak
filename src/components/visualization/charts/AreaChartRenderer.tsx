"use client"

import { Area, AreaChart, CartesianGrid, XAxis } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from "@/components/ui/chart";

interface AreaChartRendererProps {
  data: Record<string, any>[];
  xAxis: string;
  yAxis: string[];
}

export function AreaChartRenderer({ data, xAxis, yAxis }: AreaChartRendererProps) {
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
      <AreaChart data={data}>
        <defs>
          {yAxis.map((key) => (
            <linearGradient key={key} id={`fill${key}`} x1="0" y1="0" x2="0" y2="1">
              <stop
                offset="5%"
                stopColor={`var(--color-${key})`}
                stopOpacity={0.8}
              />
              <stop
                offset="95%"
                stopColor={`var(--color-${key})`}
                stopOpacity={0.1}
              />
            </linearGradient>
          ))}
        </defs>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey={xAxis}
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          tickFormatter={(value) => {
            const str = String(value);
            return str.length > 10 ? str.slice(0, 10) + "..." : str;
          }}
        />
        <ChartTooltip cursor={false} content={<ChartTooltipContent indicator="dot" />} />
        {yAxis.length > 1 && <ChartLegend content={<ChartLegendContent />} />}
        {yAxis.map((key) => (
          <Area
            key={key}
            dataKey={key}
            type="natural"
            fill={`url(#fill${key})`}
            stroke={`var(--color-${key})`}
            stackId="a"
          />
        ))}
      </AreaChart>
    </ChartContainer>
  );
}
