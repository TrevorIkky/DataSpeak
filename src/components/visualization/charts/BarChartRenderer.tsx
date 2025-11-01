"use client"

import { Bar, BarChart, CartesianGrid, XAxis } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from "@/components/ui/chart";

interface BarChartRendererProps {
  data: Record<string, any>[];
  xAxis: string;
  yAxis: string[];
}

export function BarChartRenderer({ data, xAxis, yAxis }: BarChartRendererProps) {
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
      <BarChart accessibilityLayer data={data}>
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey={xAxis}
          tickLine={false}
          tickMargin={10}
          axisLine={false}
          tickFormatter={(value) => {
            const str = String(value);
            return str.length > 10 ? str.slice(0, 10) + "..." : str;
          }}
        />
        <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
        {yAxis.length > 1 && <ChartLegend content={<ChartLegendContent />} />}
        {yAxis.map((key) => (
          <Bar
            key={key}
            dataKey={key}
            fill={`var(--color-${key})`}
            radius={4}
          />
        ))}
      </BarChart>
    </ChartContainer>
  );
}
