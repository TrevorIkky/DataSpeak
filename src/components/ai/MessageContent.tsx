import { Card } from "@/components/ui/card";
import { ChartRenderer } from "@/components/visualization/ChartRenderer";
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

interface ParsedContent {
  thinking: string;
  finalAnswer: string;
}

function parseMessageContent(content: string): ParsedContent {
  // Check if message contains "Final Answer:"
  const finalAnswerIndex = content.indexOf("Final Answer:");

  if (finalAnswerIndex === -1) {
    // No "Final Answer:" marker, treat entire content as final answer
    return {
      thinking: "",
      finalAnswer: content,
    };
  }

  // Extract thinking (everything before "Final Answer:")
  const thinking = content.substring(0, finalAnswerIndex).trim();

  // Extract final answer (everything after "Final Answer:")
  const finalAnswer = content
    .substring(finalAnswerIndex + "Final Answer:".length)
    .trim();

  return { thinking, finalAnswer };
}

export function MessageContent({ message }: MessageContentProps) {
  const hasTableData = message.tableData && message.tableData.rows.length > 0;
  const hasChartData = message.chartData && message.chartData.data.rows.length > 0;
  const hasStatisticData = message.statisticData;
  const hasMapData = message.mapData && message.mapData.geometry;

  const { thinking, finalAnswer } = parseMessageContent(message.content);

  return (
    <div className="space-y-3 max-w-full overflow-hidden">
      {/* Thinking process (collapsible) */}
      {thinking && (
        <Accordion type="single" collapsible className="w-full">
          <AccordionItem value="thinking" className="border rounded-lg">
            <AccordionTrigger className="px-4 hover:no-underline">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Brain className="h-4 w-4 text-muted-foreground" />
                <span>View thinking process</span>
              </div>
            </AccordionTrigger>
            <AccordionContent className="px-4 pb-4">
              <div className="prose prose-sm max-w-none dark:prose-invert text-muted-foreground break-words overflow-x-auto">
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
                  {thinking}
                </ReactMarkdown>
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}

      {/* Final answer */}
      {finalAnswer && (
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
            {finalAnswer}
          </ReactMarkdown>
        </div>
      )}

      {/* Table data rendering */}
      {hasTableData && message.tableData && (
        <div className="rounded-lg border overflow-hidden h-[400px] w-full">
          <DataGrid result={message.tableData} />
        </div>
      )}

      {/* Chart data rendering */}
      {hasChartData && message.chartData && (
        <div className="my-4 w-full overflow-hidden">
          <ChartRenderer
            config={message.chartData.config}
            data={message.chartData.data}
          />
        </div>
      )}

      {/* Statistic data rendering */}
      {hasStatisticData && message.statisticData && (
        <Card className="p-6 bg-primary/5 border-primary/20">
          <div className="flex items-center gap-3">
            <div className="rounded-full bg-primary/10 p-3 shrink-0">
              <TrendingUp className="h-6 w-6 text-primary" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-3xl font-bold break-words">{message.statisticData.value}</div>
              <div className="text-sm text-muted-foreground break-words">
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
          <div className="h-[400px] w-full">
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
