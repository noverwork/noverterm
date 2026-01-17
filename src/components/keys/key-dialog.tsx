import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
import { cn } from '@/lib/utils';
import { invoke } from '@tauri-apps/api/core';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { useKeyStore } from '@/lib/stores';
import { type KeyType } from '@/lib/types';
import { Plus, KeyRound } from 'lucide-react';
import { toast } from 'sonner';
import { Textarea } from '@/components/ui/textarea';

const generateKeySchema = z
  .object({
    name: z.string().min(1, 'Name is required'),
    type: z.enum(['rsa', 'ed25519', 'ecdsa'] as const),
    passphrase: z.string().optional(),
    confirmPassword: z.string().optional(),
  })
  .refine((data) => !data.passphrase || data.passphrase === data.confirmPassword, {
    message: "Passwords don't match",
    path: ['confirmPassword'],
  });

type GenerateKeyFormValues = z.infer<typeof generateKeySchema>;

const importKeySchema = z.object({
  name: z.string().min(1, 'Name is required'),
  publicKey: z.string().min(1, 'Public key is required'),
  privateKeyPath: z.string().min(1, 'Private key path is required'),
});

type ImportKeyFormValues = z.infer<typeof importKeySchema>;

interface KeyDialogProps {
  trigger?: React.ReactNode;
}

export function KeyDialog({ trigger }: KeyDialogProps) {
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<'generate' | 'import'>('generate');
  const [isSubmitting, setIsSubmitting] = useState(false);

  const generateForm = useForm<GenerateKeyFormValues>({
    resolver: zodResolver(generateKeySchema),
    defaultValues: {
      name: '',
      type: 'ed25519' as KeyType,
      passphrase: '',
      confirmPassword: '',
    },
  });

  const importForm = useForm<ImportKeyFormValues>({
    resolver: zodResolver(importKeySchema),
    defaultValues: {
      name: '',
      publicKey: '',
      privateKeyPath: '',
    },
  });

  const { addKey, importKey: importKeyAction } = useKeyStore();

  const onGenerateSubmit = async (values: GenerateKeyFormValues) => {
    setIsSubmitting(true);
    try {
      await addKey(values);
      toast.success('SSH key generated successfully');
      setOpen(false);
      generateForm.reset();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to generate key');
    } finally {
      setIsSubmitting(false);
    }
  };

  const onImportSubmit = async (values: ImportKeyFormValues) => {
    setIsSubmitting(true);
    try {
      // Get fingerprint from public key
      const fingerprint = await invoke<string>('get_key_fingerprint', {
        publicKey: values.publicKey,
      });

      // For imported keys, we'll assume no passphrase for now
      // The actual passphrase check would happen when using the key
      await importKeyAction(
        {
          name: values.name,
          publicKey: values.publicKey,
          privateKeyPath: values.privateKeyPath,
        },
        fingerprint,
        false // hasPassphrase - will be detected on actual use
      );
      toast.success('SSH key imported successfully');
      setOpen(false);
      importForm.reset();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to import key');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {trigger ?? (
          <Button>
            <Plus className="mr-2 h-4 w-4" />
            Add Key
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Add SSH Key</DialogTitle>
          <DialogDescription>
            Generate a new SSH key pair or import an existing one.
          </DialogDescription>
        </DialogHeader>

        {/* Mode Toggle */}
        <div className="border-border flex gap-2 rounded-lg border p-1">
          <button
            onClick={() => {
              setMode('generate');
            }}
            className={cn(
              'flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors',
              mode === 'generate'
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-border'
            )}
          >
            Generate New
          </button>
          <button
            onClick={() => {
              setMode('import');
            }}
            className={cn(
              'flex-1 rounded-md px-3 py-1.5 text-sm font-medium transition-colors',
              mode === 'import'
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-border'
            )}
          >
            Import Existing
          </button>
        </div>

        {mode === 'generate' ? (
          <Form {...generateForm}>
            <form onSubmit={generateForm.handleSubmit(onGenerateSubmit)} className="space-y-4">
              <FormField
                control={generateForm.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Key Name</FormLabel>
                    <FormControl>
                      <Input placeholder="e.g., My GitHub Key" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={generateForm.control}
                name="type"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Key Type</FormLabel>
                    <Select onValueChange={field.onChange} defaultValue={field.value}>
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value="ed25519">
                          <div className="flex items-center gap-2">
                            <KeyRound className="h-4 w-4" />
                            <div>
                              <div className="font-medium">Ed25519</div>
                              <div className="text-muted-foreground text-xs">
                                Modern, fast, secure (Recommended)
                              </div>
                            </div>
                          </div>
                        </SelectItem>
                        <SelectItem value="rsa">
                          <div className="flex items-center gap-2">
                            <KeyRound className="h-4 w-4" />
                            <div>
                              <div className="font-medium">RSA 4096</div>
                              <div className="text-muted-foreground text-xs">
                                Widest compatibility
                              </div>
                            </div>
                          </div>
                        </SelectItem>
                        <SelectItem value="ecdsa">
                          <div className="flex items-center gap-2">
                            <KeyRound className="h-4 w-4" />
                            <div>
                              <div className="font-medium">ECDSA</div>
                              <div className="text-muted-foreground text-xs">Balanced option</div>
                            </div>
                          </div>
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={generateForm.control}
                name="passphrase"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Passphrase (Optional)</FormLabel>
                    <FormControl>
                      <Input
                        type="password"
                        placeholder="Enter passphrase for extra security"
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              {generateForm.watch('passphrase') && (
                <FormField
                  control={generateForm.control}
                  name="confirmPassword"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Confirm Passphrase</FormLabel>
                      <FormControl>
                        <Input type="password" placeholder="Confirm passphrase" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setOpen(false);
                  }}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? 'Generating...' : 'Generate Key'}
                </Button>
              </DialogFooter>
            </form>
          </Form>
        ) : (
          <Form {...importForm}>
            <form onSubmit={importForm.handleSubmit(onImportSubmit)} className="space-y-4">
              <FormField
                control={importForm.control}
                name="name"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Key Name</FormLabel>
                    <FormControl>
                      <Input placeholder="e.g., Work Key" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={importForm.control}
                name="publicKey"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Public Key</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder="ssh-rsa AAAAB3..."
                        className="font-mono text-xs"
                        rows={3}
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={importForm.control}
                name="privateKeyPath"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Private Key Path</FormLabel>
                    <FormControl>
                      <Input placeholder="~/.ssh/id_rsa" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setOpen(false);
                  }}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting}>
                  {isSubmitting ? 'Importing...' : 'Import Key'}
                </Button>
              </DialogFooter>
            </form>
          </Form>
        )}
      </DialogContent>
    </Dialog>
  );
}
