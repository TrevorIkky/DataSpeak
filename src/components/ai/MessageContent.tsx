import { useEffect, useState } from "react";
import { Card } from "@/components/ui/card";
import { ChartRenderer } from "@/components/visualization/ChartRenderer";
import { PlotlySandbox } from "@/components/visualization/PlotlySandbox";
import { DataGrid } from "@/components/query/DataGrid";
import { MapViewer } from "@/components/map/MapViewer";
import type { ChatMessage } from "@/types/ai.types";
import ReactMarkdown from "react-markdown";
import { TrendingUp, Brain, MapPin } from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

interface MessageContentProps {
  message: ChatMessage;
}

export function MessageContent({ message }: MessageContentProps) {
  const hasTableData = message.tableData && message.tableData.rows.length > 0;
  const hasChartData = message.chartData && message.chartData.data.rows.length > 0;
  const hasPlotlyChart = message.plotlyChart && message.plotlyChart.plotlyData;
  const hasStatisticData = message.statisticData;
  const hasMapData = message.mapData && message.mapData.geometry;

  const thinking = message.thinking || "";
  const content = message.content || "";
  const isStreaming = thinking && !content;

  // Control accordion state - open while streaming, collapse when answer arrives
  const [accordionValue, setAccordionValue] = useState<string | undefined>(
    isStreaming ? "thinking" : undefined
  );

  useEffect(() => {
    if (isStreaming) {
      setAccordionValue("thinking");
    } else if (content) {
      setAccordionValue(undefined);
    }
  }, [isStreaming, content]);

  return (
    <div className="space-y-3 max-w-full overflow-hidden">
      {/* Thinking process (collapsible) - open while streaming, collapse when done */}
      {thinking && (
        <Accordion
          type="single"
          collapsible
          value={accordionValue}
          onValueChange={setAccordionValue}
          className="w-full"
        >
          <AccordionItem
            value="thinking"
            className="border rounded-lg"
          >
            <AccordionTrigger className="px-3 sm:px-4 hover:no-underline">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Brain className={`h-4 w-4 ${isStreaming ? "text-primary animate-pulse" : "text-muted-foreground"}`} />
                <span className={isStreaming ? "shine-text" : ""}>{isStreaming ? "Thinking..." : "View thinking process"}</span>
              </div>
            </AccordionTrigger>
            <AccordionContent className="px-3 sm:px-4 pb-3 sm:pb-4">
              <div className="text-xs sm:text-sm text-muted-foreground font-mono whitespace-pre-wrap">
                {thinking}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}

      {/* Main content */}
      {content && (
        <div className="prose prose-sm max-w-none dark:prose-invert break-words overflow-x-auto">
          <ReactMarkdown
            components={{
              pre: ({ node, ...props }) => (
                <pre className="overflow-x-auto" {...props} />
              ),
              code: ({ node, ...props }) => (
                <code className="break-all" {...props} />
              ),
            }}
          >
            {content}
          </ReactMarkdown>
        </div>
      )}

      {/* Table data rendering */}
      {hasTableData && message.tableData && (
        <div className="rounded-lg border overflow-hidden h-[280px] sm:h-[350px] md:h-[400px] w-full">
          <DataGrid result={message.tableData} />
        </div>
      )}

      {/* Legacy chart data rendering (Recharts) */}
      {hasChartData && message.chartData && (
        <div className="my-4 w-full overflow-hidden">
          <ChartRenderer
            config={message.chartData.config}
            data={message.chartData.data}
          />
        </div>
      )}

      {/* Plotly chart rendering (JSON data approach) */}
      {hasPlotlyChart && message.plotlyChart && (
        <div className="my-4 w-full overflow-hidden">
          <PlotlySandbox
            plotlyData={message.plotlyChart.plotlyData}
            plotlyLayout={message.plotlyChart.plotlyLayout}
            title={message.plotlyChart.title}
            chartType={message.plotlyChart.chartType}
          />
        </div>
      )}

      {/* Statistic data rendering */}
      {hasStatisticData && message.statisticData && (
        <Card className="p-4 sm:p-6 bg-primary/5 border-primary/20">
          <div className="flex items-center gap-2 sm:gap-3">
            <div className="rounded-full bg-primary/10 p-2 sm:p-3 shrink-0">
              <TrendingUp className="h-5 w-5 sm:h-6 sm:w-6 text-primary" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-2xl sm:text-3xl font-bold break-words">{message.statisticData.value}</div>
              <div className="text-xs sm:text-sm text-muted-foreground break-words">
                {message.statisticData.label}
              </div>
            </div>
          </div>
        </Card>
      )}

      {/* Map data rendering */}
      {hasMapData && message.mapData && (
        <Card className="overflow-hidden border-green-200 dark:border-green-800">
          <div className="bg-green-50 dark:bg-green-950/30 px-4 py-2 border-b border-green-200 dark:border-green-800">
            <div className="flex items-center gap-2">
              <MapPin className="h-4 w-4 text-green-600 dark:text-green-400" />
              <h3 className="text-sm font-semibold text-green-900 dark:text-green-100">
                {message.mapData.title || "Location"}
              </h3>
            </div>
            {message.mapData.description && (
              <p className="text-xs text-green-700 dark:text-green-300 mt-1">
                {message.mapData.description}
              </p>
            )}
          </div>
          <div className="h-[280px] sm:h-[350px] md:h-[400px] w-full">
            <MapViewer
              geometry={message.mapData.geometry}
              onClose={() => {}}
              isFullscreen={false}
            />
          </div>
        </Card>
      )}
    </div>
  );
}
