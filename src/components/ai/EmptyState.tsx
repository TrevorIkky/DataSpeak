import { MessageCircleMore } from "lucide-react";
import {
  Empty,
  EmptyHeader,
  EmptyMedia,
  EmptyDescription,
} from "@/components/ui/empty";

export function EmptyState() {
  return (
    <div className="flex items-center justify-center h-full">
      <Empty className="border-0">
        <EmptyHeader>
          <EmptyMedia variant="icon">
            <MessageCircleMore />
          </EmptyMedia>
          <EmptyDescription>
            Start a conversation with AI
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    </div>
  );
}
