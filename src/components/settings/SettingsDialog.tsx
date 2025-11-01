import { useEffect } from "react";
import { useForm } from "react-hook-form";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { useUIStore } from "@/stores/uiStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useTheme } from "@/components/theme-provider";
import type { AppSettings } from "@/types/settings.types";

const TEXT_TO_SQL_MODELS = [
  { id: "anthropic/claude-3-opus", name: "Claude 3 Opus" },
  { id: "anthropic/claude-3-sonnet", name: "Claude 3 Sonnet" },
  { id: "openai/gpt-4o", name: "GPT-4o" },
  { id: "google/gemini-2.0-pro", name: "Gemini 2.0 Pro" },
];

const VISUALIZATION_MODELS = [
  { id: "anthropic/claude-3-sonnet", name: "Claude 3 Sonnet" },
  { id: "openai/gpt-4o", name: "GPT-4o" },
  { id: "google/gemini-2.0-pro", name: "Gemini 2.0 Pro" },
];

export function SettingsDialog() {
  const { settingsDialogOpen, setSettingsDialogOpen } = useUIStore();
  const { settings, saveSettings, isLoading } = useSettingsStore();
  const { theme, setTheme } = useTheme();

  const form = useForm<AppSettings>({
    defaultValues: {
      openrouter_api_key: "",
      text_to_sql_model: "anthropic/claude-3-sonnet",
      visualization_model: "anthropic/claude-3-sonnet",
    },
  });

  // Load settings into form when dialog opens
  useEffect(() => {
    if (settingsDialogOpen && settings) {
      form.reset(settings);
    }
  }, [settingsDialogOpen, settings, form]);

  const onSubmit = async (data: AppSettings) => {
    await saveSettings(data);
    setSettingsDialogOpen(false);
  };

  return (
    <Dialog open={settingsDialogOpen} onOpenChange={setSettingsDialogOpen}>
      <DialogContent className="sm:max-w-[525px]">
        <DialogHeader>
          <DialogTitle>Settings</DialogTitle>
          <DialogDescription>
            Configure your theme, OpenRouter API key, and AI model preferences.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
            <div className="space-y-2">
              <label className="text-sm font-medium leading-none">
                Theme
              </label>
              <Select value={theme} onValueChange={(value: "light" | "dark" | "system") => setTheme(value)}>
                <SelectTrigger>
                  <SelectValue placeholder="Select theme" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="light">Light</SelectItem>
                  <SelectItem value="dark">Dark</SelectItem>
                  <SelectItem value="system">System</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-sm text-muted-foreground">
                Choose your preferred color theme.
              </p>
            </div>

            <Separator />

            <div className="space-y-1">
              <h4 className="text-sm font-medium">AI Configuration</h4>
              <p className="text-sm text-muted-foreground">
                Configure OpenRouter API settings and model preferences.
              </p>
            </div>

            <FormField
              control={form.control}
              name="openrouter_api_key"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>OpenRouter API Key</FormLabel>
                  <FormControl>
                    <Input
                      type="password"
                      placeholder="sk-or-v1-..."
                      {...field}
                    />
                  </FormControl>
                  <FormDescription>
                    Your OpenRouter API key for AI features. Get one at{" "}
                    <a
                      href="https://openrouter.ai"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-primary underline"
                    >
                      openrouter.ai
                    </a>
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="text_to_sql_model"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Text-to-SQL Model</FormLabel>
                  <Select onValueChange={field.onChange} value={field.value}>
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Select a model" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {TEXT_TO_SQL_MODELS.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                          {model.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <FormDescription>
                    Model used for generating SQL from natural language.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="visualization_model"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Visualization Model</FormLabel>
                  <Select onValueChange={field.onChange} value={field.value}>
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue placeholder="Select a model" />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      {VISUALIZATION_MODELS.map((model) => (
                        <SelectItem key={model.id} value={model.id}>
                          {model.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <FormDescription>
                    Model used for generating chart configurations.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setSettingsDialogOpen(false)}
                disabled={isLoading}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={isLoading}>
                {isLoading ? "Saving..." : "Save Settings"}
              </Button>
            </div>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
