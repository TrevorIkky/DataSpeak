import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Lightbulb } from "lucide-react";
import type { VisualizationConfig } from "@/types/ai.types";
import type { QueryResult } from "@/types/query.types";
import { BarChartRenderer } from "./charts/BarChartRenderer";
import { LineChartRenderer } from "./charts/LineChartRenderer";
import { PieChartRenderer } from "./charts/PieChartRenderer";
import { AreaChartRenderer } from "./charts/AreaChartRenderer";
import { ScatterChartRenderer } from "./charts/ScatterChartRenderer";
import { RadarChartRenderer } from "./charts/RadarChartRenderer";
import { RadialChartRenderer } from "./charts/RadialChartRenderer";

interface ChartRendererProps {
  config: VisualizationConfig;
  data: QueryResult;
}

export function ChartRenderer({ config, data }: ChartRendererProps) {
  const renderChart = () => {
    const chartData = data.rows;

    switch (config.type) {
      case 'bar':
        return (
          <BarChartRenderer
            data={chartData}
            xAxis={config.config.x_axis}
            yAxis={config.config.y_axis}
          />
        );

      case 'line':
        return (
          <LineChartRenderer
            data={chartData}
            xAxis={config.config.x_axis}
            yAxis={config.config.y_axis}
          />
        );

      case 'pie':
        return (
          <PieChartRenderer
            data={chartData}
            dataKey={config.config.y_axis[0]}
            nameKey={config.config.category || config.config.x_axis}
          />
        );

      case 'area':
        return (
          <AreaChartRenderer
            data={chartData}
            xAxis={config.config.x_axis}
            yAxis={config.config.y_axis}
          />
        );

      case 'scatter':
        return (
          <ScatterChartRenderer
            data={chartData}
            xAxis={config.config.x_axis}
            yAxis={config.config.y_axis[0]}
          />
        );

      case 'radar':
        return (
          <RadarChartRenderer
            data={chartData}
            xAxis={config.config.x_axis}
            yAxis={config.config.y_axis}
          />
        );

      case 'radial':
        return (
          <RadialChartRenderer
            data={chartData}
            dataKey={config.config.y_axis[0]}
            nameKey={config.config.category || config.config.x_axis}
          />
        );

      default:
        return (
          <div className="flex items-center justify-center h-[300px]">
            <p className="text-muted-foreground">Unsupported chart type: {config.type}</p>
          </div>
        );
    }
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{config.title}</CardTitle>
        {config.description && (
          <CardDescription>{config.description}</CardDescription>
        )}
      </CardHeader>
      <CardContent>
        <div className="h-[300px]">
          {renderChart()}
        </div>
      </CardContent>

      {/* AI Insights */}
      {config.insights && config.insights.length > 0 && (
        <CardContent className="pt-0">
          <div className="p-3 bg-primary/5 rounded-lg border border-primary/20">
            <div className="flex items-center gap-2 mb-2">
              <Lightbulb className="h-4 w-4 text-primary" />
              <span className="text-sm font-medium text-primary">AI Insights</span>
            </div>
            <ul className="text-sm space-y-1">
              {config.insights.map((insight, i) => (
                <li key={i} className="flex items-start gap-2">
                  <span className="text-primary mt-0.5">•</span>
                  <span>{insight}</span>
                </li>
              ))}
            </ul>
          </div>
        </CardContent>
      )}
    </Card>
  );
}
