<script lang="ts">
  import { motion } from '@humanspeak/svelte-motion';

  interface Props {
    className?: string;
    reducedMotion?: boolean;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    duration = 0.9,
    style = '',
  }: Props = $props();
</script>

<motion.div
  class={className}
  {style}
  animate={reducedMotion ? { rotate: 0 } : { rotate: 360 }}
  transition={{
    duration: reducedMotion ? 0 : duration,
    ease: 'linear',
    repeat: reducedMotion ? 0 : Infinity,
  }}
/>
