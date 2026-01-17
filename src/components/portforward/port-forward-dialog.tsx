import { useState } from 'react';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import * as z from 'zod';
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
import { useSessionStore, usePortForwardStore } from '@/lib/stores';
import { type ForwardType } from '@/lib/types';
import { Plus, ArrowRightLeft } from 'lucide-react';
import { toast } from 'sonner';

const portForwardSchema = z
  .object({
    sessionId: z.string().min(1, 'Session is required'),
    name: z.string().min(1, 'Name is required'),
    type: z.enum(['local', 'remote', 'dynamic'] as const),
    localHost: z.string().min(1, 'Local host is required'),
    localPort: z.number().int().min(1).max(65535),
    remoteHost: z.string().optional(),
    remotePort: z.number().optional(),
  })
  .refine(
    (data) => {
      // For local and remote, require remote host/port
      if (data.type === 'local' || data.type === 'remote') {
        return data.remoteHost && data.remotePort;
      }
      return true;
    },
    {
      message: 'Remote host and port are required for this type',
      path: ['remoteHost'],
    }
  );

type PortForwardFormValues = z.infer<typeof portForwardSchema>;

interface PortForwardDialogProps {
  trigger?: React.ReactNode;
  sessionId?: string;
}

export function PortForwardDialog({ trigger, sessionId: propSessionId }: PortForwardDialogProps) {
  const [open, setOpen] = useState(false);
  const { sessions } = useSessionStore();
  const { addPortForward } = usePortForwardStore();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const form = useForm<PortForwardFormValues>({
    resolver: zodResolver(portForwardSchema),
    defaultValues: {
      sessionId: propSessionId ?? '',
      name: '',
      type: 'local' as ForwardType,
      localHost: 'localhost',
      localPort: 0,
      remoteHost: 'localhost',
      remotePort: 0,
    },
  });

  const forwardType = form.watch('type');

  const onSubmit = async (values: PortForwardFormValues) => {
    setIsSubmitting(true);
    try {
      await addPortForward(values);
      toast.success('Port forward added successfully');
      setOpen(false);
      form.reset();
    } catch (error) {
      toast.error(error instanceof Error ? error.message : 'Failed to add port forward');
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
            Add Port Forward
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Add Port Forward</DialogTitle>
          <DialogDescription>
            Configure a new port forwarding rule for SSH tunneling.
          </DialogDescription>
        </DialogHeader>

        <Form {...form}>
          <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
            {!propSessionId && (
              <FormField
                control={form.control}
                name="sessionId"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Session</FormLabel>
                    <Select onValueChange={field.onChange} defaultValue={field.value}>
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Select a session" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {sessions.map((session) => (
                          <SelectItem key={session.id} value={session.id}>
                            {session.name} ({session.host})
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
            )}

            <FormField
              control={form.control}
              name="name"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Name</FormLabel>
                  <FormControl>
                    <Input placeholder="e.g., Database Tunnel" {...field} />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />

            <FormField
              control={form.control}
              name="type"
              render={({ field }) => (
                <FormItem>
                  <FormLabel>Type</FormLabel>
                  <Select
                    onValueChange={(value) => {
                      field.onChange(value as ForwardType);
                    }}
                    defaultValue={field.value}
                  >
                    <FormControl>
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                    </FormControl>
                    <SelectContent>
                      <SelectItem value="local">
                        <div className="flex items-center gap-2">
                          <ArrowRightLeft className="h-4 w-4" />
                          <div>
                            <div className="font-medium">Local (-L)</div>
                            <div className="text-muted-foreground text-xs">
                              Forward local port to remote host
                            </div>
                          </div>
                        </div>
                      </SelectItem>
                      <SelectItem value="remote">
                        <div className="flex items-center gap-2">
                          <ArrowRightLeft className="h-4 w-4 rotate-180" />
                          <div>
                            <div className="font-medium">Remote (-R)</div>
                            <div className="text-muted-foreground text-xs">
                              Forward remote port to local host
                            </div>
                          </div>
                        </div>
                      </SelectItem>
                      <SelectItem value="dynamic">
                        <div className="flex items-center gap-2">
                          <ArrowRightLeft className="h-4 w-4 rotate-90" />
                          <div>
                            <div className="font-medium">Dynamic (-D)</div>
                            <div className="text-muted-foreground text-xs">
                              SOCKS proxy (local port only)
                            </div>
                          </div>
                        </div>
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <FormMessage />
                </FormItem>
              )}
            />

            <div className="grid grid-cols-2 gap-4">
              <FormField
                control={form.control}
                name="localHost"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Local Host</FormLabel>
                    <FormControl>
                      <Input placeholder="localhost" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />

              <FormField
                control={form.control}
                name="localPort"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Local Port</FormLabel>
                    <FormControl>
                      <Input
                        type="number"
                        {...field}
                        onChange={(e) => {
                          field.onChange(parseInt(e.target.value, 10));
                        }}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>

            {forwardType !== 'dynamic' && (
              <div className="grid grid-cols-2 gap-4">
                <FormField
                  control={form.control}
                  name="remoteHost"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Remote Host</FormLabel>
                      <FormControl>
                        <Input placeholder="localhost" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />

                <FormField
                  control={form.control}
                  name="remotePort"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Remote Port</FormLabel>
                      <FormControl>
                        <Input
                          type="number"
                          {...field}
                          onChange={(e) => {
                            field.onChange(parseInt(e.target.value, 10));
                          }}
                        />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              </div>
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
                {isSubmitting ? 'Adding...' : 'Add Forward'}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
