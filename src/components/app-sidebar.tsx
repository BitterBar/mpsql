import {
  IconDatabase,
  IconFileExport,
  IconSettings,
  IconHelp,
} from "@tabler/icons-react"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"

interface AppSidebarProps {
  activeTab: string
  onTabChange: (tab: "import" | "optimize" | "env" | "help") => void
}

const navItems = [
  { id: "import" as const, title: "导入", icon: IconFileExport },
  { id: "optimize" as const, title: "优化", icon: IconSettings },
  { id: "env" as const, title: "环境", icon: IconDatabase },
]

export function AppSidebar({ activeTab, onTabChange }: AppSidebarProps) {
  return (
    <Sidebar collapsible="offcanvas">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              tooltip="MPSQL"
            >
              <a href="#" className="flex items-center gap-2 px-2">
                <IconDatabase className="size-5" />
                <span className="text-base font-semibold">MPSQL</span>
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        <SidebarMenu className="px-2">
          {navItems.map((item) => (
            <SidebarMenuItem key={item.id}>
              <SidebarMenuButton
                isActive={activeTab === item.id}
                onClick={() => onTabChange(item.id)}
                className="cursor-pointer w-full"
              >
                <item.icon className="size-5" />
                <span>{item.title}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu className="px-2">
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={activeTab === "help"}
              onClick={() => onTabChange("help")}
              className="cursor-pointer w-full"
            >
              <IconHelp className="size-5" />
              <span>帮助</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
        <div className="px-3 py-2 text-xs text-muted-foreground">
          GIS 数据导入工具 v0.1.0
        </div>
      </SidebarFooter>
    </Sidebar>
  )
}
