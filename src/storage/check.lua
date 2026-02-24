local key = KEYS[1]
local max_tokens = tonumber(ARGV[1])
local refill_rate = tonumber(ARGV[2])
local current_time = tonumber(ARGV[3])

-- get current bucket state
local bucket = redis.call('HMGET', key, 'current_tokens', 'last_refill_time')
local current_tokens = tonumber(bucket[1])
local last_refill_time = tonumber(bucket[2])

-- initialize bucket if it doesn't exist
if current_tokens == nil then
    current_tokens = max_tokens
    last_refill_time = current_time
end

-- refill logic
local elapsed = math.max(0, current_time - last_refill_time)
if elapsed > 0 then
    local new_tokens = (elapsed / 1000) * refill_rate
    current_tokens = math.min(max_tokens, current_tokens + new_tokens)
    last_refill_time = current_time
end

-- check if allowed
local allowed = 0
if current_tokens >= 1 then
    current_tokens = current_tokens - 1
    allowed = 1
end

-- update bucket state
redis.call('HMSET', key, 'current_tokens', current_tokens, 'last_refill_time', last_refill_time)
redis.call('EXPIRE', key, math.ceil(max_tokens / refill_rate) + 1)

return allowed
