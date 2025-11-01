import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Database, Info, Key, Link as LinkIcon, Check, X } from "lucide-react";
import type { Table as TableType } from "@/types/database.types";

interface TablePropertiesProps {
  table: TableType | undefined;
  tableName: string;
}

export function TableProperties({ table, tableName }: TablePropertiesProps) {
  if (!table) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8">
        <Info className="h-12 w-12 text-muted-foreground mb-3" />
        <p className="text-sm text-muted-foreground">
          No schema information available for this table
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-auto p-4 space-y-3">
      {/* Table Information Card */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Database className="h-4 w-4 text-primary" />
            Table Information
          </CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="grid grid-cols-2 gap-x-6 gap-y-2">
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-0.5">Table Name</p>
              <p className="text-sm font-semibold">{tableName}</p>
            </div>
            {table.schema && (
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-0.5">Schema</p>
                <p className="text-sm font-semibold">{table.schema}</p>
              </div>
            )}
            {table.row_count !== undefined && table.row_count !== null && (
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-0.5">Total Rows</p>
                <p className="text-sm font-semibold">{table.row_count.toLocaleString()}</p>
              </div>
            )}
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-0.5">Columns</p>
              <p className="text-sm font-semibold">{table.columns.length}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Column Details Card */}
      <Card className="flex-1 flex flex-col">
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-base">
            <Info className="h-4 w-4 text-primary" />
            Column Details
          </CardTitle>
        </CardHeader>
        <CardContent className="pt-0 flex-1 flex flex-col">
          <div className="flex-1 overflow-auto">
            <Table>
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead className="font-semibold h-9">Column</TableHead>
                  <TableHead className="font-semibold h-9">Type</TableHead>
                  <TableHead className="font-semibold w-20 text-center h-9">Nullable</TableHead>
                  <TableHead className="font-semibold w-28 h-9">Keys</TableHead>
                  <TableHead className="font-semibold h-9">Constraints</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {table.columns.map((column) => (
                  <TableRow key={column.name} className="hover:bg-muted/50">
                    <TableCell className="font-medium py-2">{column.name}</TableCell>
                    <TableCell className="py-2">
                      <code className="text-xs bg-muted px-1.5 py-0.5 rounded">
                        {column.data_type}
                        {column.character_maximum_length && ` (${column.character_maximum_length})`}
                      </code>
                    </TableCell>
                    <TableCell className="text-center py-2">
                      {column.is_nullable ? (
                        <Check className="h-3.5 w-3.5 text-green-600 mx-auto" />
                      ) : (
                        <X className="h-3.5 w-3.5 text-red-600 mx-auto" />
                      )}
                    </TableCell>
                    <TableCell className="py-2">
                      <div className="flex gap-1">
                        {column.is_primary_key && (
                          <Badge variant="default" className="bg-blue-500 hover:bg-blue-600 text-white h-5 text-xs px-1.5">
                            <Key className="h-2.5 w-2.5 mr-1" />
                            PK
                          </Badge>
                        )}
                        {column.is_foreign_key && (
                          <Badge variant="default" className="bg-purple-500 hover:bg-purple-600 text-white h-5 text-xs px-1.5">
                            <LinkIcon className="h-2.5 w-2.5 mr-1" />
                            FK
                          </Badge>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="py-2">
                      <div className="flex flex-col gap-0.5 text-xs text-muted-foreground">
                        {column.is_foreign_key && column.foreign_key_table && (
                          <div className="flex items-center gap-1">
                            <LinkIcon className="h-3 w-3" />
                            <span>
                              → {column.foreign_key_table}
                              {column.foreign_key_column && `.${column.foreign_key_column}`}
                            </span>
                          </div>
                        )}
                        {column.default_value && (
                          <div>
                            <span className="font-medium">DEFAULT:</span> {column.default_value}
                          </div>
                        )}
                        {!column.is_nullable && !column.is_primary_key && !column.is_foreign_key && !column.default_value && (
                          <span className="italic">—</span>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>

          {/* Legend */}
          <div className="mt-2 pt-2 border-t flex items-center gap-4 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Badge variant="default" className="bg-blue-500 text-white h-5 text-xs px-1.5">
                <Key className="h-2.5 w-2.5 mr-1" />
                PK
              </Badge>
              <span>Primary Key</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Badge variant="default" className="bg-purple-500 text-white h-5 text-xs px-1.5">
                <LinkIcon className="h-2.5 w-2.5 mr-1" />
                FK
              </Badge>
              <span>Foreign Key</span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
