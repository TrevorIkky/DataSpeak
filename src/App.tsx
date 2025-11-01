import { useEffect } from "react";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import { Settings, Database, Plus, Menu } from "lucide-react";
import { useUIStore } from "@/stores/uiStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { SettingsDialog } from "@/components/settings/SettingsDialog";
import { ConnectionDialog } from "@/components/connections/ConnectionDialog";
import { DatabaseNavigator } from "@/components/navigator/DatabaseNavigator";
import { QueryWorkspace } from "@/components/query/QueryWorkspace";
import { ThemeProvider } from "@/components/theme-provider";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";

function App() {
  const {
    setSettingsDialogOpen,
    setConnectionDialogOpen,
    mobileConnectionsOpen,
    setMobileConnectionsOpen,
  } = useUIStore();
  const { loadSettings } = useSettingsStore();
  const { loadConnections, activeConnection } = useConnectionStore();

  useEffect(() => {
    // Load initial data
    loadSettings();
    loadConnections();
  }, [loadSettings, loadConnections]);

  return (
    <ThemeProvider defaultTheme="system" storageKey="dataspeak-theme">
      <div className="flex h-screen w-screen flex-col bg-background">
        {/* Header */}
        <header className="flex items-center justify-between border-b px-4 py-3 bg-card">
          <div className="flex items-center gap-2 md:gap-3">
            {/* Mobile Menu Button */}
            <div className="flex md:hidden">
              <Sheet open={mobileConnectionsOpen} onOpenChange={setMobileConnectionsOpen}>
                <SheetTrigger asChild>
                  <Button variant="ghost" size="icon">
                    <Menu className="h-5 w-5" />
                  </Button>
                </SheetTrigger>
                <SheetContent side="left" className="w-80 p-0">
                  <SheetHeader className="p-4 border-b">
                    <SheetTitle>Database Navigator</SheetTitle>
                    <SheetDescription>
                      Browse your database connections and tables
                    </SheetDescription>
                  </SheetHeader>
                  <div className="h-[calc(100vh-8rem)] overflow-hidden">
                    <DatabaseNavigator />
                  </div>
                </SheetContent>
              </Sheet>
            </div>

            <Database className="h-6 w-6 text-primary" />
            <h1 className="text-lg md:text-xl font-bold">DataSpeak</h1>
            {activeConnection && (
              <div className="hidden sm:flex items-center gap-2 ml-2 md:ml-4 px-2 md:px-3 py-1 rounded-md bg-primary/10 border border-primary/20">
                <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                <span className="text-xs md:text-sm font-medium">{activeConnection.name}</span>
              </div>
            )}
          </div>

          <div className="flex items-center gap-1 md:gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setSettingsDialogOpen(true)}
              className="hidden sm:flex"
            >
              <Settings className="h-4 w-4 mr-2" />
              Settings
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={() => setSettingsDialogOpen(true)}
              className="sm:hidden"
            >
              <Settings className="h-4 w-4" />
            </Button>
            <Button
              variant="default"
              size="sm"
              onClick={() => setConnectionDialogOpen(true)}
              className="hidden sm:flex"
            >
              <Plus className="h-4 w-4 mr-2" />
              New Connection
            </Button>
            <Button
              variant="default"
              size="icon"
              onClick={() => setConnectionDialogOpen(true)}
              className="sm:hidden"
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>
        </header>

        {/* Main Content */}
        <main className="flex-1 overflow-hidden">
          <ResizablePanelGroup direction="horizontal" className="h-full">
            {/* Left Sidebar - Database Navigator */}
            <ResizablePanel
              defaultSize={25}
              minSize={20}
              maxSize={35}
              className="hidden md:flex"
            >
              <aside className="w-full border-r bg-card flex flex-col">
                <DatabaseNavigator />
              </aside>
            </ResizablePanel>

            <ResizableHandle className="hidden md:flex" withHandle />

            {/* Main Query Workspace */}
            <ResizablePanel defaultSize={80} minSize={50}>
              <QueryWorkspace />
            </ResizablePanel>
          </ResizablePanelGroup>
        </main>

        {/* Dialogs */}
        <SettingsDialog />
        <ConnectionDialog />

        {/* Toast Notifications */}
        <Toaster />
      </div>
    </ThemeProvider>
  );
}

export default App;
