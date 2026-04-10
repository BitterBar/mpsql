import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { BrowserRouter, Routes, Route, Navigate, useLocation, useNavigate } from "react-router-dom";
import {
  Settings,
  RefreshCw,
  Play,
} from "lucide-react";
import { SiteHeader } from "@/components/site-header";
import { AppSidebar } from "@/components/app-sidebar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar";
import { LogProvider, useLogs } from "@/contexts/LogContext";
import { ConnectionProvider, useConnection } from "@/contexts/ConnectionContext";
import { AppStateProvider, useAppState } from "@/contexts/AppStateContext";
import { DatabaseSelector } from "@/components/database-selector";
import { LogPanel } from "@/components/log-panel";

interface EnvInfo {
  path: string;
  exists: boolean;
  packages: string[];
}

function ImportPage() {
  const { selectedConnection } = useConnection();
  const { addLog } = useLogs();
  const [envInfo, setEnvInfo] = useState<EnvInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [inputPath, setInputPath] = useState("");
  const [layerName, setLayerName] = useState("");
  const [importSchema, setImportSchema] = useState("public");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [sourceSrs, setSourceSrs] = useState("");
  const [targetSrs, setTargetSrs] = useState("EPSG:4326");
  const [geometryName, setGeometryName] = useState("geom");
  const [fidColumn, setFidColumn] = useState("");
  const [selectFields, setSelectFields] = useState("");
  const [encoding, setEncoding] = useState("UTF-8");
  const [overwrite, setOverwrite] = useState(false);
  const [promoteToMulti, setPromoteToMulti] = useState(false);
  const [skipFailures, setSkipFailures] = useState(false);
  const [useCopy, setUseCopy] = useState(true);

  useEffect(() => {
    checkEnvStatus();
  }, []);

  async function checkEnvStatus() {
    try {
      const info = await invoke<EnvInfo>("check_env_status");
      setEnvInfo(info);
    } catch (e) {
      console.error("检查环境状态失败:", e);
    }
  }

  async function selectFile() {
    const selected = await open({
      multiple: false,
      filters: [
        { name: "GIS Files", extensions: ["shp", "gpkg", "geojson", "json", "kml", "gml"] },
        { name: "All Files", extensions: ["*"] },
      ],
    });
    if (selected) setInputPath(selected as string);
  }

  async function selectFolder() {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      setInputPath(selected as string);
      setLayerName("");
    }
  }

  async function runOgrConvert() {
    if (!inputPath || !selectedConnection) {
      addLog("warning", "请选择文件和数据库连接");
      return;
    }

    setLoading(true);
    addLog("info", "开始导入数据...", inputPath);

    const connStr = `PG:host=${selectedConnection.host} port=${selectedConnection.port} dbname=${selectedConnection.database} user=${selectedConnection.user} password=${selectedConnection.password}`;

    try {
      const result = await invoke<string>("ogr_convert", {
        options: {
          input_path: inputPath,
          output_connection: connStr,
          layer_name: layerName || null,
          srs: sourceSrs || null,
          target_srs: targetSrs || null,
          schema: importSchema || null,
          geometry_name: geometryName || null,
          fid_column: fidColumn || null,
          overwrite,
          promote_to_multi: promoteToMulti,
          select_fields: selectFields || null,
          skip_failures: skipFailures,
          encoding: encoding || null,
          use_copy: useCopy,
        },
      });
      addLog("success", "导入完成", result);
    } catch (e) {
      addLog("error", "导入失败", String(e));
    }
    setLoading(false);
  }

  if (!envInfo?.exists) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>环境设置</CardTitle>
          <CardDescription>创建 GDAL 环境以开始使用</CardDescription>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">请先在"环境"页面创建 GDAL 环境</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>导入 GIS 数据</CardTitle>
          <CardDescription>使用 GDAL 将 GIS 文件导入 PostgreSQL</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            <Input
              value={inputPath}
              onChange={(e) => setInputPath(e.target.value)}
              placeholder="请选择文件或文件夹..."
              className="flex-1"
            />
            <Button variant="outline" onClick={selectFile}>
              选择文件
            </Button>
            <Button variant="outline" onClick={selectFolder}>
              选择文件夹
            </Button>
          </div>
          <div className="grid grid-cols-4 items-center gap-4">
            <Label className="text-right">图层名称</Label>
            <Input
              className="col-span-3"
              placeholder="可选的图层名称"
              value={layerName}
              onChange={(e) => setLayerName(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-4 items-center gap-4">
            <Label className="text-right">Schema</Label>
            <Input
              className="col-span-3"
              placeholder="public"
              value={importSchema}
              onChange={(e) => setImportSchema(e.target.value)}
            />
          </div>
          <Button
            variant="ghost"
            size="sm"
            className="text-slate-500 block pl-0"
            onClick={() => setShowAdvanced(!showAdvanced)}
          >
            {showAdvanced ? "隐藏" : "显示"}高级选项
          </Button>
          {showAdvanced && (
            <div className="space-y-3 p-4 border rounded-lg">
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">源坐标系</Label>
                <Input
                  className="col-span-3"
                  placeholder="EPSG:4490"
                  value={sourceSrs}
                  onChange={(e) => setSourceSrs(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">目标坐标系</Label>
                <Input
                  className="col-span-3"
                  placeholder="EPSG:4326"
                  value={targetSrs}
                  onChange={(e) => setTargetSrs(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">几何列名</Label>
                <Input
                  className="col-span-3"
                  placeholder="geom"
                  value={geometryName}
                  onChange={(e) => setGeometryName(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">FID 列名</Label>
                <Input
                  className="col-span-3"
                  placeholder="id"
                  value={fidColumn}
                  onChange={(e) => setFidColumn(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">选择字段</Label>
                <Input
                  className="col-span-3"
                  placeholder="field1,field2,..."
                  value={selectFields}
                  onChange={(e) => setSelectFields(e.target.value)}
                />
              </div>
              <div className="grid grid-cols-4 items-center gap-4">
                <Label className="text-right text-sm">编码</Label>
                <Input
                  className="col-span-3"
                  placeholder="UTF-8"
                  value={encoding}
                  onChange={(e) => setEncoding(e.target.value)}
                />
              </div>
              <div className="flex flex-wrap gap-4">
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="overwrite"
                    checked={overwrite}
                    onCheckedChange={(c) => setOverwrite(!!c)}
                  />
                  <Label htmlFor="overwrite" className="text-sm">
                    覆盖
                  </Label>
                </div>
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="promoteMulti"
                    checked={promoteToMulti}
                    onCheckedChange={(c) => setPromoteToMulti(!!c)}
                  />
                  <Label htmlFor="promoteMulti" className="text-sm">
                    转为 Multi
                  </Label>
                </div>
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="skipFail"
                    checked={skipFailures}
                    onCheckedChange={(c) => setSkipFailures(!!c)}
                  />
                  <Label htmlFor="skipFail" className="text-sm">
                    跳过错误
                  </Label>
                </div>
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="useCopy"
                    checked={useCopy}
                    onCheckedChange={(c) => setUseCopy(!!c)}
                  />
                  <Label htmlFor="useCopy" className="text-sm">
                    使用 COPY 模式
                  </Label>
                </div>
              </div>
            </div>
          )}
          <Button
            onClick={runOgrConvert}
            disabled={loading || !selectedConnection || !inputPath}
          >
            <Play className="h-4 w-4 mr-2" />
            {loading ? "导入中..." : "导入到数据库"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}

function OptimizePage() {
  const { selectedConnection } = useConnection();
  const { addLog } = useLogs();
  const [loading, setLoading] = useState(false);
  const [optimizeSchema, setOptimizeSchema] = useState("public");
  const [optimizeTable, setOptimizeTable] = useState("");
  const [createGeomIndex, setCreateGeomIndex] = useState(true);

  async function runOptimize() {
    if (!selectedConnection) {
      addLog("warning", "请选择数据库连接");
      return;
    }

    setLoading(true);
    addLog("info", "开始优化数据库...", optimizeSchema);

    const connStr = `PG:host=${selectedConnection.host} port=${selectedConnection.port} dbname=${selectedConnection.database} user=${selectedConnection.user} password=${selectedConnection.password}`;

    try {
      const result = await invoke<string>("optimize_postgres", {
        options: {
          connection: connStr,
          schema: optimizeSchema || null,
          table: optimizeTable || null,
          create_geometry_index: createGeomIndex,
        },
      });
      addLog("success", "优化完成", result);
    } catch (e) {
      addLog("error", "优化失败", String(e));
    }
    setLoading(false);
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>优化数据库</CardTitle>
        <CardDescription>运行 ANALYZE、创建索引和 VACUUM</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid grid-cols-4 items-center gap-4">
          <Label className="text-right">Schema</Label>
          <Input
            className="col-span-3"
            value={optimizeSchema}
            onChange={(e) => setOptimizeSchema(e.target.value)}
          />
        </div>
        <div className="grid grid-cols-4 items-center gap-4">
          <Label className="text-right">表名</Label>
          <Input
            className="col-span-3"
            placeholder="留空则优化所有表"
            value={optimizeTable}
            onChange={(e) => setOptimizeTable(e.target.value)}
          />
        </div>
        <div className="flex items-center space-x-2">
          <Checkbox
            id="geomIndex"
            checked={createGeomIndex}
            onCheckedChange={(checked) => setCreateGeomIndex(!!checked)}
          />
          <Label htmlFor="geomIndex">创建几何索引 (GIST)</Label>
        </div>
        <Button onClick={runOptimize} disabled={loading || !selectedConnection}>
          <Settings className="h-4 w-4 mr-2" />
          {loading ? "优化中..." : "开始优化"}
        </Button>
      </CardContent>
    </Card>
  );
}

function EnvPage() {
  const { addLog } = useLogs();
  const { envLoading, setEnvLoading, setEnvNeedsRefresh } = useAppState();
  const [envInfo, setEnvInfo] = useState<EnvInfo | null>(null);

  useEffect(() => {
    checkEnvStatus();
  }, []);

  async function checkEnvStatus() {
    try {
      const info = await invoke<EnvInfo>("check_env_status");
      setEnvInfo(info);
    } catch (e) {
      console.error("检查环境状态失败:", e);
    }
  }

  async function createEnv() {
    setEnvLoading({ isLoading: true, message: "正在创建 GDAL 环境..." });
    addLog("info", "正在创建 GDAL 环境...");
    try {
      await invoke<string>("create_env", {
        packages: ["gdal", "libpq", "libgdal-pg"],
      });
      await checkEnvStatus();
      addLog("success", "环境创建完成");
      setEnvLoading({ isLoading: false, message: "" });
      setEnvNeedsRefresh(true);
    } catch (e) {
      addLog("error", "环境创建失败", String(e));
      setEnvLoading({ isLoading: false, message: String(e) });
    }
  }

  const loading = envLoading.isLoading;

  return (
    <Card>
      <CardHeader>
        <CardTitle>GDAL 环境</CardTitle>
        <CardDescription>管理 GDAL 运行环境和依赖包</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <div className="font-medium text-lg">
              {envInfo?.exists ? "环境已就绪" : "环境未创建"}
            </div>
            <div className="text-sm text-muted-foreground">
              {envInfo?.exists ? envInfo.path : "点击按钮创建 GDAL 环境"}
            </div>
          </div>
          <Button onClick={createEnv} disabled={loading} size="sm">
            <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
            {loading ? "创建中..." : (envInfo?.exists ? "重新创建" : "创建环境")}
          </Button>
        </div>

        {envInfo?.exists && (
          <div className="border rounded-lg p-4">
            <div className="text-sm font-medium mb-3">已安装的包</div>
            <div className="flex flex-wrap gap-2">
              {envInfo.packages.map((pkg) => (
                <span
                  key={pkg}
                  className="px-3 py-1 bg-muted rounded-full text-xs font-medium"
                >
                  {pkg}
                </span>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function HelpPage() {
  const formats = [
    { ext: "SHP", name: "Shapefile", desc: "ESRI Shapefile 格式" },
    { ext: "GPKG", name: "GeoPackage", desc: "OGC GeoPackage 格式" },
    { ext: "GeoJSON", name: "GeoJSON", desc: "RFC 7946 GeoJSON 格式" },
    { ext: "JSON", name: "JSON", desc: "通用 JSON 格式" },
    { ext: "KML", name: "KML", desc: "Google Earth KML 格式" },
    { ext: "GML", name: "GML", desc: "Geography Markup Language" },
  ];

  return (
    <Card>
      <CardHeader>
        <CardTitle>支持的格式</CardTitle>
        <CardDescription>GDAL 支持导入的矢量数据格式</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid grid-cols-2 gap-3">
          {formats.map((f) => (
            <div key={f.ext} className="flex items-center gap-3 p-3 rounded-lg border">
              <span className="px-2 py-1 bg-muted rounded text-xs font-mono font-medium">
                {f.ext}
              </span>
              <div>
                <div className="font-medium text-sm">{f.name}</div>
                <div className="text-xs text-muted-foreground">{f.desc}</div>
              </div>
            </div>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function AppContent() {
  const location = useLocation();
  const navigate = useNavigate();
  const { envNeedsRefresh, setEnvNeedsRefresh } = useAppState();
  const [envInfo, setEnvInfo] = useState<EnvInfo | null>(null);
  const [gdalInstalled, setGdalInstalled] = useState(false);

  const activeTab = location.pathname.replace("/", "") || "import";

  async function checkEnvStatus() {
    try {
      const info = await invoke<EnvInfo>("check_env_status");
      setEnvInfo(info);
      if (info.exists) {
        try {
          const hasGdal = await invoke<boolean>("check_gdal");
          setGdalInstalled(hasGdal);
        } catch {
          setGdalInstalled(false);
        }
      } else {
        setGdalInstalled(false);
      }
    } catch (e) {
      console.error("检查环境状态失败:", e);
    }
  }

  useEffect(() => {
    checkEnvStatus();
  }, []);

  useEffect(() => {
    if (envNeedsRefresh) {
      checkEnvStatus();
      setEnvNeedsRefresh(false);
    }
  }, [envNeedsRefresh]);

  return (
    <>
      <SidebarProvider>
        <AppSidebar
          activeTab={activeTab}
          onTabChange={(tab) => navigate(`/${tab}`)}
        />
        <SidebarInset>
          <SiteHeader
            gdalInstalled={gdalInstalled}
            envExists={envInfo?.exists}
            onRefresh={checkEnvStatus}
          >
            <DatabaseSelector />
          </SiteHeader>
          <div className="flex flex-1 flex-col overflow-hidden overscroll-none">
            <div className="flex-1 gap-4 p-4 overflow-y-auto overscroll-none">
              <Routes>
                <Route path="/" element={<Navigate to="/import" replace />} />
                <Route path="/import" element={<ImportPage />} />
                <Route path="/optimize" element={<OptimizePage />} />
                <Route path="/env" element={<EnvPage />} />
                <Route path="/help" element={<HelpPage />} />
              </Routes>
            </div>
            <LogPanel />
          </div>
        </SidebarInset>
      </SidebarProvider>
    </>
  );
}

function App() {
  return (
    <BrowserRouter>
      <AppStateProvider>
        <LogProvider>
          <ConnectionProvider>
            <AppContent />
          </ConnectionProvider>
        </LogProvider>
      </AppStateProvider>
    </BrowserRouter>
  );
}

export default App;
