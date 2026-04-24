<script lang="ts">
  import { motion } from '@humanspeak/svelte-motion';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';

  type MotionTarget = Record<string, string | number>;

  interface Props {
    reducedMotion: boolean;
  }

  const PANEL_DURATION = 0.18;
  const HERO_DURATION = 0.22;
  const HERO_DELAY = 0.03;
  const LIST_DURATION = 0.2;
  const LIST_DELAY = 0.07;

  let props: Props = $props();

  function motionTransition(duration: number, delay = 0): any {
    const transition: any = {
      duration: props.reducedMotion ? 0 : duration,
      delay: props.reducedMotion ? 0 : delay,
      ease: 'easeOut' as const,
    };

    return transition;
  }

  function fadeEnter(opacity = 0): MotionTarget {
    return props.reducedMotion ? { opacity: 1 } : { opacity };
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }
</script>

<motion.div
  class="album-detail-card"
  initial={fadeEnter()}
  animate={{ opacity: 1 }}
  exit={fadeExit()}
  transition={motionTransition(PANEL_DURATION)}
>
  <div class="album-hero">
    <motion.div
      class="album-hero-info"
      initial={fadeEnter()}
      animate={{ opacity: 1 }}
      exit={fadeExit()}
      transition={motionTransition(HERO_DURATION, HERO_DELAY)}
    >
      <MotionPulseBlock
        className="album-hero-title loading-text"
        reducedMotion={props.reducedMotion}
      />
      <MotionPulseBlock
        className="album-hero-sub loading-text-sub"
        reducedMotion={props.reducedMotion}
        delay={0.14}
      />
    </motion.div>
  </div>
  <motion.div
    class="loading album-loading"
    initial={fadeEnter()}
    animate={{ opacity: 1 }}
    exit={fadeExit()}
    transition={motionTransition(LIST_DURATION, LIST_DELAY)}
  >
    <span>正在加载歌曲...</span><MotionSpinner
      className="inline-loading-spinner"
      reducedMotion={props.reducedMotion}
    />
  </motion.div>
</motion.div>
