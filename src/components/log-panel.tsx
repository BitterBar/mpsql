import { useState } from "react";
import { ChevronUp, ChevronDown, Trash2, Info, CheckCircle, AlertTriangle, XCircle } from "lucide-react";
import { useLogs, LogLevel } from "@/contexts/LogContext";
import { Button } from "@/components/ui/button";

const levelIcons: Record<LogLevel, React.ReactNode> = {
  info: <Info className="h-3 w-3 text-blue-500" />,
  success: <CheckCircle className="h-3 w-3 text-green-500" />,
  warning: <AlertTriangle className="h-3 w-3 text-yellow-500" />,
  error: <XCircle className="h-3 w-3 text-red-500" />,
};

const levelColors: Record<LogLevel, string> = {
  info: "bg-blue-50 border-blue-200",
  success: "bg-green-50 border-green-200",
  warning: "bg-yellow-50 border-yellow-200",
  error: "bg-red-50 border-red-200",
};

export function LogPanel() {
  const { logs, clearLogs } = useLogs();
  const [expanded, setExpanded] = useState(false);

  if (logs.length === 0) return null;

  return (
    <div className="border-t bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 mt-auto shrink-0 overflow-hidden">
      <div className="flex items-center justify-between px-4 py-1 border-b bg-muted/30">
        <span className="text-xs text-muted-foreground">
          日志 ({logs.length})
        </span>
        <div className="flex gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronUp className="h-3 w-3" />
            )}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-2 text-xs"
            onClick={clearLogs}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </div>
      {expanded && (
        <div className="max-h-[200px] overflow-y-auto p-2 space-y-1">
          {logs.map((log) => (
            <div
              key={log.id}
              className={`flex flex-wrap items-start gap-2 px-2 py-1 rounded text-xs border ${levelColors[log.level]}`}
            >
              {levelIcons[log.level]}
              <div className="flex-1 min-w-0">
                <div className="font-medium break-all">{log.message}</div>
                {log.details && (
                  <div className="text-muted-foreground break-all whitespace-pre-wrap">
                    {log.details}
                  </div>
                )}
              </div>
              <span className="text-muted-foreground shrink-0">
                {log.timestamp.toLocaleTimeString()}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
