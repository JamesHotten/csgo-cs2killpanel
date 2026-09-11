using System;
using System.Collections.Generic;

namespace KillConfirmGameBar.Danmaku.Engine
{
    internal sealed class DanmakuBatchComposer
    {
        private readonly DanmakuWeightEngine _weightEngine;
        private readonly Random _random;

        public DanmakuBatchComposer(Random random)
        {
            _random = random ?? new Random();
            _weightEngine = new DanmakuWeightEngine(_random);
        }

        public IReadOnlyList<DanmakuMessage> Compose(DanmakuEventContext context, int visibleLimit)
        {
            if (context == null)
            {
                return new DanmakuMessage[0];
            }

            DanmakuReactionPolicy policy = DanmakuReactionPolicies.Resolve(context.Kind);
            int targetCount = Math.Max(1, Math.Min(DanmakuReactionPolicies.EventMaximumVisibleCount, visibleLimit));
            int coreCount = Math.Min(DanmakuReactionPolicies.EventBurstCount, targetCount);
            DateTimeOffset now = DateTimeOffset.UtcNow;
            DanmakuEventDynamics dynamics = DanmakuEventSemantics.ResolveDynamics(
                context.Kind,
                DanmakuSettingsStore.EventIntensity);
            var result = new List<DanmakuMessage>(targetCount);
            var eventHistory = new DanmakuSelectionHistory();
            for (int i = 0; i < targetCount; i++)
            {
                DanmakuMessageRole role = i < coreCount
                    ? DanmakuMessageRole.Core
                    : DanmakuMessageRole.Atmosphere;
                DanmakuSelectionResult selection = _weightEngine.SelectEventDanmaku(
                    context.Kind,
                    eventHistory,
                    role);
                if (selection == null || !selection.IsSuccess)
                {
                    break;
                }

                result.Add(new DanmakuMessage
                {
                    Text = selection.Text,
                    Role = role,
                    EventPriority = policy.Priority,
                    IsEventReaction = true,
                    NotBefore = now.AddSeconds(dynamics.ResolveDispatchOffsetSeconds(i, targetCount)),
                    ExpiresAt = now.AddSeconds(DanmakuReactionPolicies.EventDurationSeconds)
                });
            }

            return result;
        }
    }
}
