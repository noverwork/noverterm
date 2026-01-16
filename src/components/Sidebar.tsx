import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { Button } from '@/components/ui/button';
import { Plus, Server, FolderTree, Settings, Terminal } from 'lucide-react';
import { cn } from '@/lib/utils';

interface SidebarProps {
  className?: string;
}

export function Sidebar({ className }: SidebarProps) {
  return (
    <div className={cn('flex h-full flex-col border-r border-slate-800 bg-slate-900', className)}>
      {/* Header */}
      <div className="p-4">
        <div className="flex items-center gap-2 px-2">
          <div className="rounded-lg bg-blue-600 p-2">
            <Terminal className="h-4 w-4 text-white" />
          </div>
          <div>
            <h1 className="font-semibold text-slate-100">Noverterm</h1>
            <p className="text-xs text-slate-500">SSH Terminal Manager</p>
          </div>
        </div>
      </div>

      <Separator className="bg-slate-800" />

      {/* New Connection Button */}
      <div className="p-4">
        <Button className="w-full justify-start gap-2 bg-blue-600 hover:bg-blue-700">
          <Plus className="h-4 w-4" />
          New SSH Connection
        </Button>
      </div>

      {/* Navigation */}
      <ScrollArea className="flex-1 px-4">
        <nav className="space-y-6 py-4">
          {/* Connections Section */}
          <div>
            <h3 className="mb-2 px-2 text-xs font-semibold tracking-wider text-slate-500 uppercase">
              SSH Connections
            </h3>
            <div className="space-y-1">
              <button className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-slate-400 transition-colors hover:bg-slate-800 hover:text-slate-100">
                <Server className="h-4 w-4" />
                <span className="text-sm">server-01</span>
                <div className="ml-auto h-2 w-2 rounded-full bg-green-500" />
              </button>
              <button className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-slate-400 transition-colors hover:bg-slate-800 hover:text-slate-100">
                <Server className="h-4 w-4" />
                <span className="text-sm">server-02</span>
                <div className="ml-auto h-2 w-2 rounded-full bg-slate-600" />
              </button>
            </div>
          </div>

          {/* Groups Section */}
          <div>
            <h3 className="mb-2 px-2 text-xs font-semibold tracking-wider text-slate-500 uppercase">
              Groups
            </h3>
            <div className="space-y-1">
              <button className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-slate-400 transition-colors hover:bg-slate-800 hover:text-slate-100">
                <FolderTree className="h-4 w-4" />
                <span className="text-sm">Production</span>
              </button>
              <button className="flex w-full items-center gap-3 rounded-lg px-2 py-2 text-slate-400 transition-colors hover:bg-slate-800 hover:text-slate-100">
                <FolderTree className="h-4 w-4" />
                <span className="text-sm">Development</span>
              </button>
            </div>
          </div>
        </nav>
      </ScrollArea>

      {/* Footer */}
      <div className="border-t border-slate-800 p-4">
        <Button
          variant="ghost"
          className="w-full justify-start gap-2 text-slate-400 hover:text-slate-100"
        >
          <Settings className="h-4 w-4" />
          Settings
        </Button>
      </div>
    </div>
  );
}
