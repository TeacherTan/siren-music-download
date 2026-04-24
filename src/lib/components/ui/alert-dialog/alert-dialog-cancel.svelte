<script lang="ts">
  import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';

  type ButtonVariant =
    | 'default'
    | 'outline'
    | 'secondary'
    | 'ghost'
    | 'destructive'
    | 'link';
  type ButtonSize =
    | 'default'
    | 'xs'
    | 'sm'
    | 'lg'
    | 'icon'
    | 'icon-xs'
    | 'icon-sm'
    | 'icon-lg';

  function resolveButtonClasses(variant: ButtonVariant, size: ButtonSize) {
    const base =
      'inline-flex items-center justify-center rounded-lg border border-transparent text-sm font-medium transition-all outline-none select-none';
    const variantClass =
      variant === 'outline'
        ? 'border-border bg-background hover:bg-muted'
        : variant === 'secondary'
          ? 'bg-secondary text-secondary-foreground'
          : variant === 'ghost'
            ? 'hover:bg-muted'
            : variant === 'destructive'
              ? 'bg-destructive/10 text-destructive'
              : variant === 'link'
                ? 'text-primary underline-offset-4 hover:underline'
                : 'bg-primary text-primary-foreground';
    const sizeClass =
      size === 'sm' ? 'h-7 px-2.5' : size === 'lg' ? 'h-9 px-3' : 'h-8 px-2.5';

    return `${base} ${variantClass} ${sizeClass}`;
  }

  let {
    ref = $bindable(null),
    class: className,
    variant = 'outline',
    size = 'default',
    ...restProps
  }: AlertDialogPrimitive.CancelProps & {
    variant?: ButtonVariant;
    size?: ButtonSize;
  } = $props();
</script>

<AlertDialogPrimitive.Cancel
  bind:ref
  data-slot="alert-dialog-cancel"
  class={cn(
    resolveButtonClasses(variant, size),
    'cn-alert-dialog-cancel',
    className
  )}
  {...restProps}
/>
