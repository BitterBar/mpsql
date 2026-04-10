import { useState } from "react";
import { Database, ChevronDown, Check, Plus, Trash2, Loader2, XCircle, AlertCircle } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { useConnection } from "@/contexts/ConnectionContext";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { invoke } from "@tauri-apps/api/core";

export function DatabaseSelector() {
  const {
    connections,
    selectedConnection,
    selectConnection,
    refreshConnections,
  } = useConnection();
  const [open, setOpen] = useState(false);
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ status: "success" | "error"; message: string } | null>(null);
  const [newConn, setNewConn] = useState({
    name: "",
    host: "localhost",
    port: "5432",
    database: "",
    user: "postgres",
    password: "",
  });

  async function testConnection() {
    if (!newConn.database || !newConn.host) {
      setTestResult({ status: "error", message: "请填写完整信息" });
      return;
    }
    
    setTesting(true);
    setTestResult(null);
    
    try {
      const result = await invoke<string>("test_connection", {
        connection: { ...newConn, id: "" },
      });
      setTestResult({ status: "success", message: result });
    } catch (e) {
      setTestResult({ status: "error", message: String(e) });
    }
    
    setTesting(false);
  }

  async function saveConnection() {
    try {
      const id = crypto.randomUUID();
      await invoke("save_connection", {
        connection: { ...newConn, id },
      });
      await refreshConnections();
      selectConnection(id);
      setShowAddDialog(false);
      setNewConn({
        name: "",
        host: "localhost",
        port: "5432",
        database: "",
        user: "postgres",
        password: "",
      });
      setTestResult(null);
    } catch (e) {
      console.error(`保存连接失败: ${e}`);
    }
  }

  async function deleteConnection(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await invoke("delete_connection", { id });
      await refreshConnections();
    } catch (e) {
      console.error(`删除连接失败: ${e}`);
    }
  }

  function handleDialogClose(open: boolean) {
    setShowAddDialog(open);
    if (!open) {
      setTestResult(null);
    }
  }

  return (
    <>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            variant="outline"
            className="min-w-50 justify-between font-normal border rounded-md h-8"
          >
            <span className="flex items-center gap-2">
              <Database className="h-4 w-4" />
              {selectedConnection ? (
                <>
                  <span className="font-medium">{selectedConnection.name}</span>
                  <span className="text-muted-foreground text-xs">
                    ({selectedConnection.database})
                  </span>
                </>
              ) : (
                <span className="text-muted-foreground">选择数据库...</span>
              )}
            </span>
            <ChevronDown className="h-4 w-4" />
          </Button>
        </PopoverTrigger>
        <PopoverContent className="w-70 p-0" align="start">
          <div className="border-b px-3 py-2">
            <p className="text-xs text-muted-foreground">已保存的连接</p>
          </div>
          <div className="max-h-60 overflow-y-auto">
            {connections.map((conn) => (
              <div
                key={conn.id}
                role="button"
                tabIndex={0}
                onClick={() => {
                  selectConnection(conn.id);
                  setOpen(false);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    selectConnection(conn.id);
                    setOpen(false);
                  }
                }}
                className="w-full px-3 py-2 text-left hover:bg-accent flex items-center justify-between group cursor-pointer"
              >
                <div>
                  <div className="font-medium text-sm">{conn.name}</div>
                  <div className="text-xs text-muted-foreground">
                    {conn.host}:{conn.port}/{conn.database}
                  </div>
                </div>
                <div className="flex items-center gap-1">
                  {selectedConnection?.id === conn.id && (
                    <Check className="h-4 w-4 text-green-500" />
                  )}
                  <button
                    onClick={(e) => deleteConnection(conn.id, e)}
                    className="opacity-0 group-hover:opacity-100 p-1 hover:text-red-500 transition-opacity"
                  >
                    <Trash2 className="h-3 w-3" />
                  </button>
                </div>
              </div>
            ))}
            {connections.length === 0 && (
              <div className="px-3 py-4 text-center text-sm text-muted-foreground">
                暂无保存的连接
              </div>
            )}
          </div>
          <div className="border-t p-2">
            <Button
              variant="ghost"
              size="sm"
              className="w-full justify-start text-muted-foreground"
              onClick={() => {
                setOpen(false);
                setShowAddDialog(true);
              }}
            >
              <Plus className="h-4 w-4 mr-2" />
              添加新连接
            </Button>
          </div>
        </PopoverContent>
      </Popover>

      <Dialog open={showAddDialog} onOpenChange={handleDialogClose}>
        <DialogContent className="sm:max-w-100">
          <DialogHeader>
            <DialogTitle>添加数据库连接</DialogTitle>
            <DialogDescription>请输入 PostgreSQL 连接信息</DialogDescription>
          </DialogHeader>
          <div className="grid gap-3 py-4">
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">名称</Label>
              <Input
                className="col-span-3 text-sm"
                placeholder="我的数据库"
                autoComplete="off"
                value={newConn.name}
                onChange={(e) =>
                  setNewConn({ ...newConn, name: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">主机</Label>
              <Input
                className="col-span-3 text-sm"
                autoComplete="off"
                value={newConn.host}
                onChange={(e) =>
                  setNewConn({ ...newConn, host: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">端口</Label>
              <Input
                className="col-span-3 text-sm"
                autoComplete="off"
                value={newConn.port}
                onChange={(e) =>
                  setNewConn({ ...newConn, port: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">数据库</Label>
              <Input
                className="col-span-3 text-sm"
                autoComplete="off"
                value={newConn.database}
                onChange={(e) =>
                  setNewConn({ ...newConn, database: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">用户</Label>
              <Input
                className="col-span-3 text-sm"
                autoComplete="off"
                value={newConn.user}
                onChange={(e) =>
                  setNewConn({ ...newConn, user: e.target.value })
                }
              />
            </div>
            <div className="grid grid-cols-4 items-center gap-2">
              <Label className="text-right text-xs">密码</Label>
              <Input
                className="col-span-3 text-sm"
                type="password"
                autoComplete="off"
                value={newConn.password}
                onChange={(e) =>
                  setNewConn({ ...newConn, password: e.target.value })
                }
              />
            </div>
          </div>
          
          {testResult && (
            <div className={`flex items-center gap-2 text-sm px-1 ${testResult.status === "success" ? "text-green-600" : "text-red-600"}`}>
              {testResult.status === "success" ? (
                <Check className="h-4 w-4 shrink-0" />
              ) : (
                <XCircle className="h-4 w-4 shrink-0" />
              )}
              <span className="truncate">{testResult.message}</span>
            </div>
          )}
          
          <DialogFooter>
            <Button 
              variant="outline" 
              size="sm" 
              onClick={testConnection}
              disabled={testing || !newConn.database || !newConn.host}
            >
              {testing ? (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              ) : (
                <AlertCircle className="h-4 w-4 mr-2" />
              )}
              测试连接
            </Button>
            <Button size="sm" onClick={saveConnection}>
              保存
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
