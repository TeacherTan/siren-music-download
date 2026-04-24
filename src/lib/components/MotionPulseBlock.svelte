<script lang="ts">
  import { motion } from '@humanspeak/svelte-motion';

  interface Props {
    className?: string;
    reducedMotion?: boolean;
    delay?: number;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    delay = 0,
    duration = 1.8,
    style = '',
  }: Props = $props();
</script>

<motion.div
  class={className}
  {style}
  animate={reducedMotion ? { opacity: 1 } : { opacity: [0.46, 0.92, 0.46] }}
  transition={{
    duration: reducedMotion ? 0 : duration,
    ease: 'easeInOut',
    repeat: reducedMotion ? 0 : Infinity,
    delay: reducedMotion ? 0 : delay,
  }}
/>
