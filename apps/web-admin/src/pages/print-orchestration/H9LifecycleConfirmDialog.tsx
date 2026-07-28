import {
  Button,
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@wms/ui";

export interface H9LifecycleConfirmation {
  title: string;
  description: string;
  confirmLabel: string;
  destructive?: boolean;
}

interface H9LifecycleConfirmDialogProps {
  confirmation: H9LifecycleConfirmation | null;
  pending: boolean;
  errorMessage?: string;
  onOpenChange: (open: boolean) => void;
  onConfirm: () => void;
}

export function H9LifecycleConfirmDialog({
  confirmation,
  pending,
  errorMessage,
  onOpenChange,
  onConfirm,
}: H9LifecycleConfirmDialogProps) {
  return (
    <Dialog open={Boolean(confirmation)} onOpenChange={(next) => !pending && onOpenChange(next)}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{confirmation?.title ?? "确认操作"}</DialogTitle>
          <DialogDescription>{confirmation?.description ?? "请确认本次操作。"}</DialogDescription>
        </DialogHeader>
        {errorMessage && (
          <p className="text-sm text-destructive" role="alert">
            {errorMessage}
          </p>
        )}
        <DialogFooter>
          <DialogClose asChild>
            <Button type="button" variant="outline" disabled={pending}>
              取消
            </Button>
          </DialogClose>
          <Button
            type="button"
            variant={confirmation?.destructive ? "destructive" : "default"}
            disabled={pending || !confirmation}
            onClick={onConfirm}
          >
            {pending ? "处理中..." : confirmation?.confirmLabel ?? "确认"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
