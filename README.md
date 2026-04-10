# MPSQL - GIS 数据导入工具

基于 Tauri v2 的 GIS 数据导入工具，支持通过 ogr2ogr 将 GIS 数据导入 PostgreSQL 数据库。

## 功能特性

- **环境管理** - 自动管理 GDAL 运行环境和依赖包（gdal、libpq、postgresql）
- **多格式支持** - 支持导入 SHP、GeoPackage、GeoJSON、KML、GML 等常见 GIS 格式
- **批量导入** - 支持文件夹批量导入所有 .shp 文件
- **数据库管理** - 支持保存多个数据库连接，快速切换
- **连接测试** - 新建连接时支持测试数据库连通性
- **数据库优化** - 一键运行 ANALYZE、创建几何索引、VACUUM
- **跨平台** - 支持 macOS (ARM/Intel)、Linux、Windows

## 技术栈

- **前端**: React + TypeScript + Vite
- **UI**: shadcn/ui + Tailwind CSS
- **后端**: Rust + Tauri v2
- **GIS**: GDAL (通过 micromamba 管理)

## 支持的文件格式

| 格式 | 扩展名 | 说明 |
|------|--------|------|
| Shapefile | .shp | ESRI Shapefile 格式 |
| GeoPackage | .gpkg | OGC GeoPackage 格式 |
| GeoJSON | .geojson, .json | RFC 7946 GeoJSON 格式 |
| KML | .kml | Google Earth KML 格式 |
| GML | .gml | Geography Markup Language |

## 开发

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建
pnpm tauri build
```

## 项目结构

```
mpsql/
├── src/                    # 前端源代码
│   ├── components/        # React 组件
│   ├── contexts/          # React Context
│   └── App.tsx            # 主应用
├── src-tauri/             # Rust 后端源代码
│   ├── src/lib.rs         # 主要业务逻辑
│   └── binaries/          # micromamba 二进制文件
└── dist/                  # 构建输出
```

## 使用说明

1. **创建 GDAL 环境** - 首次使用需在"环境"页面创建 GDAL 环境
2. **添加数据库连接** - 在顶部数据库选择器中添加 PostgreSQL 连接
3. **导入数据** - 选择文件或文件夹，点击"导入到数据库"
4. **优化数据库** - 导入完成后可在"优化"页面创建索引和 VACUUM

## License

MIT
