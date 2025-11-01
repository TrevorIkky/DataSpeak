import { useState, useEffect } from "react";
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
import { Loader2, CheckCircle2 } from "lucide-react";
import { useUIStore } from "@/stores/uiStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useSchemaStore } from "@/stores/schemaStore";
import type { IConnectionFormData } from "@/interfaces/connection.interface";
import type { DatabaseType } from "@/types/database.types";

export function ConnectionDialog() {
  const { connectionDialogOpen, setConnectionDialogOpen, editingConnectionId } = useUIStore();
  const { connections, saveConnection, updateConnection, testConnection, setActiveConnection, isLoading } = useConnectionStore();
  const { loadSchema } = useSchemaStore();
  const [isTesting, setIsTesting] = useState(false);
  const [testStatus, setTestStatus] = useState<"idle" | "success" | "error">("idle");

  const isEditMode = !!editingConnectionId;
  const editingConnection = isEditMode ? connections.find(c => c.id === editingConnectionId) : null;

  const form = useForm<IConnectionFormData>({
    defaultValues: {
      name: "",
      database_type: "PostgreSQL" as DatabaseType,
      host: "localhost",
      port: 5432,
      username: "",
      password: "",
      default_database: "",
    },
  });

  // Pre-fill form when editing
  useEffect(() => {
    if (isEditMode && editingConnection) {
      form.reset({
        name: editingConnection.name,
        database_type: editingConnection.database_type,
        host: editingConnection.host,
        port: editingConnection.port,
        username: editingConnection.username,
        password: editingConnection.password,
        default_database: editingConnection.default_database,
      });
    }
  }, [isEditMode, editingConnection, form]);

  // Update default port when database type changes
  const handleDatabaseTypeChange = (type: DatabaseType) => {
    const defaultPorts: Record<DatabaseType, number> = {
      PostgreSQL: 5432,
      MariaDB: 3306,
      MySQL: 3306,
    };
    form.setValue("port", defaultPorts[type]);
  };

  const onTestConnection = async () => {
    setIsTesting(true);
    setTestStatus("idle");

    try {
      const data = form.getValues();
      const testData = {
        id: "",
        ...data,
        created_at: "",
        updated_at: "",
      };

      const result = await testConnection(testData);

      if (result.success) {
        setTestStatus("success");
      } else {
        setTestStatus("error");
      }
    } catch (error) {
      setTestStatus("error");
    } finally {
      setIsTesting(false);
    }
  };

  const onSubmit = async (data: IConnectionFormData) => {
    try {
      let savedConnection: any;

      if (isEditMode && editingConnectionId) {
        // Update existing connection
        savedConnection = await updateConnection(editingConnectionId, data);
      } else {
        // Create new connection
        const connection = {
          id: "",
          ...data,
          created_at: "",
          updated_at: "",
        };
        savedConnection = await saveConnection(connection);

        // Only set as active connection when creating new
        setActiveConnection(savedConnection);
      }

      // Close dialog and reset immediately after successful save
      setConnectionDialogOpen(false);
      form.reset();
      setTestStatus("idle");

      // Load schema in the background (don't block dialog close)
      if (!isEditMode) {
        try {
          await loadSchema(savedConnection.id);
        } catch (schemaError) {
          console.error("Failed to load schema:", schemaError);
          // Schema loading failure shouldn't prevent dialog from closing
        }
      }
    } catch (error) {
      console.error(isEditMode ? "Failed to update connection:" : "Failed to save connection:", error);
      // Error is already handled by the store
    }
  };

  return (
    <Dialog
      open={connectionDialogOpen}
      onOpenChange={(open) => {
        setConnectionDialogOpen(open);
        if (!open) {
          form.reset();
          setTestStatus("idle");
          setIsTesting(false);
        }
      }}
    >
      <DialogContent className="sm:max-w-[600px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{isEditMode ? "Edit Database Connection" : "New Database Connection"}</DialogTitle>
          <DialogDescription>
            {isEditMode
              ? "Update your database connection settings."
              : "Configure a connection to your PostgreSQL, MariaDB, or MySQL database."}
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-6">
            <div className="grid grid-cols-2 gap-4">
              <FormField
                control={form.control}
                name="name"
                rules={{ required: "Connection name is required" }}
                render={({ field }) => (
                  <FormItem className="col-span-2">
                    <FormLabel>Connection Name *</FormLabel>
                    <FormControl>
                      <Input placeholder="My Database" {...field} />
                    </FormControl>
                    <FormDescription>
                      A friendly name to identify this connection
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="database_type"
                render={({ field }) => (
                  <FormItem className="col-span-2">
                    <FormLabel>Database Type *</FormLabel>
                    <Select
                      onValueChange={(value) => {
                        field.onChange(value);
                        handleDatabaseTypeChange(value as DatabaseType);
                      }}
                      value={field.value}
                    >
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Select database type" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="PostgreSQL">PostgreSQL</SelectItem>
                        <SelectItem value="MariaDB">MariaDB</SelectItem>
                        <SelectItem value="MySQL">MySQL</SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="host"
                rules={{ required: "Host is required" }}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Host *</FormLabel>
                    <FormControl>
                      <Input placeholder="localhost" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="port"
                rules={{
                  required: "Port is required",
                  min: { value: 1, message: "Port must be greater than 0" },
                  max: { value: 65535, message: "Port must be less than 65536" }
                }}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Port *</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        {...field}
                        onChange={(e) => field.onChange(parseInt(e.target.value))}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="username"
                rules={{ required: "Username is required" }}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Username *</FormLabel>
                    <FormControl>
                      <Input placeholder="postgres" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="password"
                rules={{ required: "Password is required" }}
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Password *</FormLabel>
                    <FormControl>
                      <Input type="password" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="default_database"
                rules={{ required: "Database name is required" }}
                render={({ field }) => (
                  <FormItem className="col-span-2">
                    <FormLabel>Database Name *</FormLabel>
                    <FormControl>
                      <Input placeholder="myapp_db" {...field} />
                    </FormControl>
                    <FormDescription>
                      The default database to connect to
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            <div className="flex items-center justify-between pt-4 border-t">
              <Button
                type="button"
                variant="outline"
                onClick={onTestConnection}
                disabled={isTesting || isLoading}
              >
                {isTesting ? (
                  <>
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    Testing...
                  </>
                ) : testStatus === "success" ? (
                  <>
                    <CheckCircle2 className="h-4 w-4 mr-2 text-green-500" />
                    Connected
                  </>
                ) : (
                  "Test Connection"
                )}
              </Button>

              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setConnectionDialogOpen(false);
                    form.reset();
                    setTestStatus("idle");
                  }}
                  disabled={isLoading}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isLoading}>
                  {isLoading ? (
                    <>
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                      {isEditMode ? "Updating..." : "Saving..."}
                    </>
                  ) : (
                    isEditMode ? "Update Connection" : "Save & Connect"
                  )}
                </Button>
              </div>
            </div>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
