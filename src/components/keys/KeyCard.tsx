import type { SSHKey } from '@/lib/types';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { useState } from 'react';
import { MoreVertical, KeyRound, Copy, Trash2, Download, Lock } from 'lucide-react';
import { toast } from 'sonner';
import { Textarea } from '@/components/ui/textarea';

const typeConfig: Record<SSHKey['type'], { label: string; color: string }> = {
  ed25519: { label: 'Ed25519', color: 'bg-blue-500/20 text-blue-400' },
  rsa: { label: 'RSA', color: 'bg-purple-500/20 text-purple-400' },
  ecdsa: { label: 'ECDSA', color: 'bg-green-500/20 text-green-400' },
};

interface KeyCardProps {
  sshKey: SSHKey;
  onDelete: (id: string) => void;
}

export function KeyCard({ sshKey, onDelete }: KeyCardProps) {
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);
  const [showPublicKeyDialog, setShowPublicKeyDialog] = useState(false);

  const typeInfo = typeConfig[sshKey.type];

  const handleCopyPublicKey = () => {
    void navigator.clipboard.writeText(sshKey.publicKey);
    toast.success('Public key copied to clipboard');
  };

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  };

  return (
    <>
      <Card className="group border-border hover:border-primary/50 transition-all duration-200">
        <div className="p-4">
          {/* Header */}
          <div className="mb-3 flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="bg-primary/20 flex h-10 w-10 items-center justify-center rounded-lg">
                <KeyRound className="text-primary h-5 w-5" />
              </div>
              <div>
                <h3 className="text-foreground font-semibold">{sshKey.name}</h3>
                <div className="flex items-center gap-2">
                  <Badge variant="secondary" className={typeInfo.color}>
                    {typeInfo.label}
                  </Badge>
                  {sshKey.hasPassphrase && (
                    <Badge variant="outline" className="gap-1">
                      <Lock className="h-3 w-3" />
                      Passphrase
                    </Badge>
                  )}
                </div>
              </div>
            </div>

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 opacity-0 transition-opacity group-hover:opacity-100"
                >
                  <MoreVertical className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem
                  onClick={() => {
                    setShowPublicKeyDialog(true);
                  }}
                >
                  <Copy className="mr-2 h-4 w-4" />
                  Copy Public Key
                </DropdownMenuItem>
                <DropdownMenuItem>
                  <Download className="mr-2 h-4 w-4" />
                  Export Private Key
                </DropdownMenuItem>
                <DropdownMenuItem
                  className="text-destructive"
                  onClick={() => {
                    setShowDeleteDialog(true);
                  }}
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>

          {/* Key Info */}
          <div className="mb-4 space-y-2 text-sm">
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">Fingerprint</span>
              <code className="text-foreground text-xs">
                {sshKey.fingerprint || sshKey.publicKey.slice(0, 16) + '...'}
              </code>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-muted-foreground">Created</span>
              <span className="text-foreground">{formatDate(sshKey.createdAt)}</span>
            </div>
          </div>

          {/* Actions */}
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setShowPublicKeyDialog(true);
            }}
            className="border-border w-full"
          >
            <Copy className="mr-2 h-3.5 w-3.5" />
            View Public Key
          </Button>
        </div>
      </Card>

      {/* Public Key Dialog */}
      <AlertDialog open={showPublicKeyDialog} onOpenChange={setShowPublicKeyDialog}>
        <AlertDialogContent className="sm:max-w-lg">
          <AlertDialogHeader>
            <AlertDialogTitle>Public Key</AlertDialogTitle>
            <AlertDialogDescription>
              Copy this public key to the target server's ~/.ssh/authorized_keys file
            </AlertDialogDescription>
          </AlertDialogHeader>

          <div className="space-y-4">
            <Textarea readOnly value={sshKey.publicKey} className="font-mono text-xs" rows={6} />
            <Button variant="outline" className="w-full" onClick={handleCopyPublicKey}>
              <Copy className="mr-2 h-4 w-4" />
              Copy to Clipboard
            </Button>
          </div>

          <AlertDialogFooter>
            <AlertDialogCancel>Close</AlertDialogCancel>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Delete Confirmation Dialog */}
      <AlertDialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete SSH Key</AlertDialogTitle>
            <AlertDialogDescription>
              Are you sure you want to delete the SSH key "{sshKey.name}"? This action cannot be
              undone. Make sure you have a backup of the private key if needed.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                onDelete(sshKey.id);
                setShowDeleteDialog(false);
              }}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
