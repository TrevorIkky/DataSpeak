import { Code, TrendingUp, MessageSquare, Lightbulb, CheckCircle } from "lucide-react";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useAiStore } from "@/stores/aiStore";
import type { AiMode } from "@/types/ai.types";

const AI_MODES: { value: AiMode; label: string; icon: React.ElementType }[] = [
  { value: 'sql', label: 'SQL', icon: Code },
  { value: 'analyst', label: 'Analyze', icon: TrendingUp },
  { value: 'explain', label: 'Explain', icon: MessageSquare },
  { value: 'insights', label: 'Insights', icon: Lightbulb },
  { value: 'quality', label: 'Quality', icon: CheckCircle },
];

export function AiModeSelector() {
  const { currentMode, setMode } = useAiStore();

  return (
    <Tabs value={currentMode} onValueChange={(value) => setMode(value as AiMode)}>
      <TabsList className="grid w-full grid-cols-2">
        {AI_MODES.slice(0, 2).map(({ value, label, icon: Icon }) => (
          <TabsTrigger key={value} value={value} className="text-xs">
            <Icon className="h-3.5 w-3.5 mr-1.5" />
            {label}
          </TabsTrigger>
        ))}
      </TabsList>

      <TabsList className="grid w-full grid-cols-3 mt-2">
        {AI_MODES.slice(2).map(({ value, label, icon: Icon }) => (
          <TabsTrigger key={value} value={value} className="text-xs">
            <Icon className="h-3.5 w-3.5 mr-1" />
            {label}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
