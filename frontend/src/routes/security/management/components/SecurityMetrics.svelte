<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte';
	import { writable } from 'svelte/store';
	
	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	// Component state
	let metrics = writable({});
	let loading = true;
	let error = null;
	let selectedTimeRange = '24h';
	let autoRefresh = true;
	
	// Time range options
	const timeRanges = [
		{ value: '1h', label: 'Last Hour' },
		{ value: '24h', label: 'Last 24 Hours' },
		{ value: '7d', label: 'Last 7 Days' },
		{ value: '30d', label: 'Last 30 Days' }
	];
	
	onMount(async () => {
		await loadMetrics();
		setupAutoRefresh();
	});
	
	async function loadMetrics() {
		try {
			loading = true;
			error = null;
			
			const response = await fetch(`/api/security/metrics?time_range=${selectedTimeRange}`);
			if (!response.ok) {
				if (response.status === 404) {
					throw new Error('Security metrics endpoint not implemented yet');
				} else {
					throw new Error(`Failed to load security metrics: ${response.status} ${response.statusText}`);
				}
			}
			
			const data = await response.json();
			if (!data.data) {
				throw new Error('Invalid metrics response: missing data field');
			}
			
			// Only apply minimal null safety for actual API responses
			// Don't create fake data, just prevent crashes on undefined properties
			const metricsData = data.data || {};
			const safeMetrics = {
				...metricsData,
				// Only add null safety checks, not fake values
				health: metricsData.health ? {
					...metricsData.health,
					last_check: metricsData.health.last_check || new Date().toISOString()
				} : null,
				performance: metricsData.performance || null,
				security: metricsData.security || null,
				audit: metricsData.audit || null,
				emergency: metricsData.emergency || null,
				configuration: metricsData.configuration || null,
				recent_alerts: metricsData.recent_alerts || [],
				top_tools: metricsData.top_tools || [],
				top_patterns: metricsData.top_patterns || [],
				top_users: metricsData.top_users || []
			};
			
			metrics.set(safeMetrics);
			
		} catch (err) {
			error = err.message;
			console.error('Failed to load security metrics:', err);
			// Don't provide fake data - let the UI show the error state
			metrics.set({});
		} finally {
			loading = false;
		}
	}
	
	function setupAutoRefresh() {
		if (autoRefresh) {
			const interval = setInterval(async () => {
				if (autoRefresh && !loading) {
					await loadMetrics();
				}
			}, 30000); // Refresh every 30 seconds
			
			return () => clearInterval(interval);
		}
	}
	
	async function changeTimeRange(range) {
		selectedTimeRange = range;
		await loadMetrics();
	}
	
	function toggleAutoRefresh() {
		autoRefresh = !autoRefresh;
		if (autoRefresh) {
			setupAutoRefresh();
		}
	}
	
	function formatNumber(num) {
		if (num >= 1000000) {
			return (num / 1000000).toFixed(1) + 'M';
		} else if (num >= 1000) {
			return (num / 1000).toFixed(1) + 'K';
		} else {
			return num?.toString() || '0';
		}
	}
	
	function formatPercentage(value, total) {
		if (!total || total === 0) return '0%';
		return Math.round((value / total) * 100) + '%';
	}
	
	function formatDuration(ms) {
		if (ms < 1000) {
			return Math.round(ms) + 'ms';
		} else if (ms < 60000) {
			return (ms / 1000).toFixed(1) + 's';
		} else {
			return Math.round(ms / 60000) + 'min';
		}
	}
	
	function getHealthStatusClass(status) {
		const classes = {
			healthy: 'health-healthy',
			warning: 'health-warning',
			critical: 'health-critical',
			unknown: 'health-unknown'
		};
		return classes[status] || 'health-unknown';
	}
	
	function getHealthStatusIcon(status) {
		const icons = {
			healthy: '🟢',
			warning: '🟡',
			critical: '🔴',
			unknown: '⚪'
		};
		return icons[status] || '⚪';
	}
</script>

<div class="security-metrics">
	<div class="header">
		<h2>📈 Security Metrics & Analytics</h2>
		<div class="header-controls">
			<div class="time-range-selector">
				<label>Time Range:</label>
				<select bind:value={selectedTimeRange} on:change={() => changeTimeRange(selectedTimeRange)}>
					{#each timeRanges as range}
						<option value={range.value}>{range.label}</option>
					{/each}
				</select>
			</div>
			
			<button 
				class="btn btn-toggle {autoRefresh ? 'active' : ''}"
				on:click={toggleAutoRefresh}
			>
				{autoRefresh ? '⏸️ Pause' : '▶️ Auto-Refresh'}
			</button>
			
			<button class="btn btn-primary" on:click={loadMetrics} disabled={loading}>
				{loading ? '🔄 Loading...' : '🔄 Refresh'}
			</button>
		</div>
	</div>

	{#if loading}
		<div class="loading">
			<div class="spinner"></div>
			<p>Loading security metrics...</p>
		</div>
	{:else if error}
		<div class="error">
			<h3>⚠️ Error Loading Metrics</h3>
			<p>{error}</p>
			<button class="btn btn-primary" on:click={loadMetrics}>Retry</button>
		</div>
	{:else}
		<!-- System Health Overview -->
		<div class="health-section">
			<h3>🏥 System Health Overview</h3>
			<div class="health-grid">
				{#if $metrics.health}
					<div class="health-card">
						<div class="health-header">
							<span class="health-icon {getHealthStatusClass($metrics.health?.overall_status)}">
								{getHealthStatusIcon($metrics.health?.overall_status)}
							</span>
							<h4>Overall Health</h4>
						</div>
						<div class="health-status">{$metrics.health?.overall_status || 'Unknown'}</div>
						<div class="health-detail">Last check: {$metrics.health?.last_check ? new Date($metrics.health?.last_check).toLocaleTimeString() : 'Unknown'}</div>
					</div>
					
					<div class="health-card">
						<div class="health-header">
							<span class="health-icon {getHealthStatusClass($metrics.health?.allowlist_status)}">
								{getHealthStatusIcon($metrics.health?.allowlist_status)}
							</span>
							<h4>Allowlist Service</h4>
						</div>
						<div class="health-status">{$metrics.health?.allowlist_status || 'Unknown'}</div>
						<div class="health-detail">
							{formatNumber($metrics.performance?.evaluations_per_second || 0)}/sec
						</div>
					</div>
					
					<div class="health-card">
						<div class="health-header">
							<span class="health-icon {getHealthStatusClass($metrics.health?.audit_status)}">
								{getHealthStatusIcon($metrics.health?.audit_status)}
							</span>
							<h4>Audit System</h4>
						</div>
						<div class="health-status">{$metrics.health?.audit_status || 'Unknown'}</div>
						<div class="health-detail">
							{formatNumber($metrics.audit?.total_events || 0)} events
						</div>
					</div>
					
					<div class="health-card">
						<div class="health-header">
							<span class="health-icon {getHealthStatusClass($metrics.health?.emergency_status)}">
								{getHealthStatusIcon($metrics.health?.emergency_status)}
							</span>
							<h4>Emergency System</h4>
						</div>
						<div class="health-status">{$metrics.health?.emergency_status || 'Unknown'}</div>
						<div class="health-detail">
							{$metrics.emergency?.is_active ? 'LOCKDOWN ACTIVE' : 'Normal Operation'}
						</div>
					</div>
				{/if}
			</div>
		</div>

		<!-- Performance Metrics -->
		{#if $metrics.performance}
			<div class="metrics-section">
				<h3>⚡ Performance Metrics</h3>
				<div class="metrics-grid">
					<div class="metric-card">
						<h4>🏃 Rule Evaluations</h4>
						<div class="metric-value">{formatNumber($metrics.performance?.total_evaluations || 0)}</div>
						<div class="metric-label">Total evaluations</div>
						<div class="metric-rate">
							{formatNumber($metrics.performance?.evaluations_per_second || 0)}/sec
						</div>
					</div>
					
					<div class="metric-card">
						<h4>⏱️ Response Time</h4>
						<div class="metric-value">{formatDuration($metrics.performance?.average_response_time || 0)}</div>
						<div class="metric-label">Average response</div>
						<div class="metric-rate">
							P95: {formatDuration($metrics.performance?.p95_response_time || 0)}
						</div>
					</div>
					
					<div class="metric-card">
						<h4>💾 Cache Hit Rate</h4>
						<div class="metric-value">
							{formatPercentage($metrics.performance?.cache_hits || 0, ($metrics.performance?.cache_hits || 0) + ($metrics.performance?.cache_misses || 0))}
						</div>
						<div class="metric-label">Cache efficiency</div>
						<div class="metric-rate">
							{formatNumber($metrics.performance?.cache_hits || 0)} hits
						</div>
					</div>
					
					<div class="metric-card">
						<h4>🎯 Success Rate</h4>
						<div class="metric-value">
							{formatPercentage($metrics.performance?.successful_evaluations || 0, $metrics.performance?.total_evaluations || 0)}
						</div>
						<div class="metric-label">Evaluation success</div>
						<div class="metric-rate">
							{formatNumber($metrics.performance?.failed_evaluations || 0)} failures
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Security Statistics -->
		{#if $metrics.security}
			<div class="metrics-section">
				<h3>🔒 Security Statistics</h3>
				<div class="security-stats-grid">
					<div class="stat-group">
						<h4>🛡️ Access Control</h4>
						<div class="stat-items">
							<div class="stat-item">
								<span class="stat-label">Allowed Requests:</span>
								<span class="stat-value allowed">{formatNumber($metrics.security?.allowed_requests || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Denied Requests:</span>
								<span class="stat-value denied">{formatNumber($metrics.security?.denied_requests || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Blocked Requests:</span>
								<span class="stat-value blocked">{formatNumber($metrics.security?.blocked_requests || 0)}</span>
							</div>
						</div>
					</div>
					
					<div class="stat-group">
						<h4>📊 Rule Usage</h4>
						<div class="stat-items">
							<div class="stat-item">
								<span class="stat-label">Tool Rules Applied:</span>
								<span class="stat-value">{formatNumber($metrics.security?.tool_rules_applied || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Pattern Rules Applied:</span>
								<span class="stat-value">{formatNumber($metrics.security?.pattern_rules_applied || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Default Decisions:</span>
								<span class="stat-value">{formatNumber($metrics.security?.default_decisions || 0)}</span>
							</div>
						</div>
					</div>
					
					<div class="stat-group">
						<h4>⚠️ Security Events</h4>
						<div class="stat-items">
							<div class="stat-item">
								<span class="stat-label">High Severity:</span>
								<span class="stat-value critical">{formatNumber($metrics.security?.high_severity_events || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Medium Severity:</span>
								<span class="stat-value warning">{formatNumber($metrics.security?.medium_severity_events || 0)}</span>
							</div>
							<div class="stat-item">
								<span class="stat-label">Low Severity:</span>
								<span class="stat-value info">{formatNumber($metrics.security?.low_severity_events || 0)}</span>
							</div>
						</div>
					</div>
				</div>
			</div>
		{/if}

		<!-- Top Tools and Patterns -->
		{#if $metrics.top_tools || $metrics.top_patterns}
			<div class="metrics-section">
				<h3>🔝 Usage Analytics</h3>
				<div class="analytics-grid">
					{#if $metrics.top_tools}
						<div class="analytics-card">
							<h4>🛠️ Most Accessed Tools</h4>
							<div class="top-list">
								{#each $metrics.top_tools as tool}
									<div class="top-item">
										<span class="item-name">{tool.name}</span>
										<span class="item-count">{formatNumber(tool.count)}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
					
					{#if $metrics.top_patterns}
						<div class="analytics-card">
							<h4>🎯 Most Triggered Patterns</h4>
							<div class="top-list">
								{#each $metrics.top_patterns as pattern}
									<div class="top-item">
										<span class="item-name">{pattern.name}</span>
										<span class="item-count">{formatNumber(pattern.count)}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
					
					{#if $metrics.top_users}
						<div class="analytics-card">
							<h4>👥 Most Active Users</h4>
							<div class="top-list">
								{#each $metrics.top_users as user}
									<div class="top-item">
										<span class="item-name">{user.user_id}</span>
										<span class="item-count">{formatNumber(user.requests)}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			</div>
		{/if}

		<!-- Recent Alerts -->
		{#if $metrics.recent_alerts && $metrics.recent_alerts.length > 0}
			<div class="metrics-section">
				<h3>🚨 Recent Security Alerts</h3>
				<div class="alerts-list">
					{#each $metrics.recent_alerts as alert}
						<div class="alert-item {alert.severity}">
							<div class="alert-header">
								<span class="alert-type">{alert.type}</span>
								<span class="alert-time">{new Date(alert.timestamp).toLocaleString()}</span>
							</div>
							<div class="alert-message">{alert.message}</div>
							{#if alert.details}
								<div class="alert-details">{alert.details}</div>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Configuration Summary -->
		{#if $metrics.configuration}
			<div class="metrics-section">
				<h3>⚙️ Configuration Summary</h3>
				<div class="config-grid">
					<div class="config-card">
						<h4>📋 Active Rules</h4>
						<div class="config-stats">
							<div class="config-item">
								<span class="config-label">Tool Rules:</span>
								<span class="config-value">{$metrics.configuration?.active_tool_rules || 0}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Capability Patterns:</span>
								<span class="config-value">{$metrics.configuration?.active_capability_patterns || 0}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Global Patterns:</span>
								<span class="config-value">{$metrics.configuration?.active_global_patterns || 0}</span>
							</div>
						</div>
					</div>
					
					<div class="config-card">
						<h4>⚠️ System Conflicts</h4>
						<div class="config-stats">
							<div class="config-item">
								<span class="config-label">Rule Conflicts:</span>
								<span class="config-value warning">{$metrics.configuration?.rule_conflicts || 0}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Pattern Overlaps:</span>
								<span class="config-value info">{$metrics.configuration?.pattern_overlaps || 0}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Disabled Rules:</span>
								<span class="config-value">{$metrics.configuration?.disabled_rules || 0}</span>
							</div>
						</div>
					</div>
					
					<div class="config-card">
						<h4>📊 Audit Configuration</h4>
						<div class="config-stats">
							<div class="config-item">
								<span class="config-label">Log Level:</span>
								<span class="config-value">{$metrics.configuration?.audit_log_level || 'Unknown'}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Retention Days:</span>
								<span class="config-value">{$metrics.configuration?.audit_retention_days || 0}</span>
							</div>
							<div class="config-item">
								<span class="config-label">Events Today:</span>
								<span class="config-value">{formatNumber($metrics.configuration?.events_today || 0)}</span>
							</div>
						</div>
					</div>
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.security-metrics {
		padding: 20px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 30px;
	}

	.header h2 {
		margin: 0;
		color: #1f2937;
	}

	.header-controls {
		display: flex;
		align-items: center;
		gap: 15px;
	}

	.time-range-selector {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.time-range-selector label {
		font-weight: 600;
		color: #374151;
		font-size: 0.9em;
	}

	.time-range-selector select {
		padding: 6px 10px;
		border: 1px solid #d1d5db;
		border-radius: 4px;
		font-size: 0.9em;
	}

	.btn {
		padding: 8px 16px;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.btn-primary {
		background: #2563eb;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #1d4ed8;
	}

	.btn-toggle {
		background: #6b7280;
		color: white;
	}

	.btn-toggle:hover {
		background: #4b5563;
	}

	.btn-toggle.active {
		background: #16a34a;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.loading {
		text-align: center;
		padding: 60px 20px;
	}

	.spinner {
		width: 40px;
		height: 40px;
		border: 4px solid #f3f4f6;
		border-top: 4px solid #2563eb;
		border-radius: 50%;
		animation: spin 1s linear infinite;
		margin: 0 auto 20px;
	}

	@keyframes spin {
		0% { transform: rotate(0deg); }
		100% { transform: rotate(360deg); }
	}

	.error {
		text-align: center;
		padding: 40px 20px;
		background: #fef2f2;
		border: 1px solid #fecaca;
		border-radius: 8px;
		color: #dc2626;
	}

	.metrics-section {
		margin-bottom: 30px;
	}

	.metrics-section h3 {
		margin: 0 0 20px 0;
		color: #1f2937;
		font-size: 1.3em;
	}

	.health-section {
		margin-bottom: 30px;
	}

	.health-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 20px;
	}

	.health-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
		text-align: center;
		transition: transform 0.2s, box-shadow 0.2s;
	}

	.health-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
	}

	.health-header {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		margin-bottom: 15px;
	}

	.health-icon {
		font-size: 1.5em;
	}

	.health-header h4 {
		margin: 0;
		color: #374151;
		font-size: 1em;
	}

	.health-status {
		font-size: 1.2em;
		font-weight: bold;
		margin-bottom: 8px;
		text-transform: capitalize;
	}

	.health-healthy .health-status {
		color: #16a34a;
	}

	.health-warning .health-status {
		color: #f59e0b;
	}

	.health-critical .health-status {
		color: #dc2626;
	}

	.health-detail {
		color: #6b7280;
		font-size: 0.9em;
	}

	.metrics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
		gap: 20px;
	}

	.metric-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
		text-align: center;
	}

	.metric-card h4 {
		margin: 0 0 15px 0;
		color: #6b7280;
		font-size: 0.9em;
	}

	.metric-value {
		font-size: 2em;
		font-weight: bold;
		color: #111827;
		margin-bottom: 5px;
	}

	.metric-label {
		color: #6b7280;
		font-size: 0.9em;
		margin-bottom: 8px;
	}

	.metric-rate {
		color: #2563eb;
		font-size: 0.8em;
		font-weight: 600;
	}

	.security-stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 20px;
	}

	.stat-group {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
	}

	.stat-group h4 {
		margin: 0 0 15px 0;
		color: #374151;
		font-size: 1em;
	}

	.stat-items {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.stat-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.stat-label {
		color: #6b7280;
		font-size: 0.9em;
	}

	.stat-value {
		font-weight: 600;
		color: #374151;
	}

	.stat-value.allowed {
		color: #16a34a;
	}

	.stat-value.denied {
		color: #dc2626;
	}

	.stat-value.blocked {
		color: #f59e0b;
	}

	.stat-value.critical {
		color: #dc2626;
	}

	.stat-value.warning {
		color: #f59e0b;
	}

	.stat-value.info {
		color: #2563eb;
	}

	.analytics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 20px;
	}

	.analytics-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
	}

	.analytics-card h4 {
		margin: 0 0 15px 0;
		color: #374151;
	}

	.top-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.top-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 0;
		border-bottom: 1px solid #f3f4f6;
	}

	.top-item:last-child {
		border-bottom: none;
	}

	.item-name {
		color: #374151;
		font-family: monospace;
		font-size: 0.9em;
	}

	.item-count {
		color: #2563eb;
		font-weight: 600;
	}

	.alerts-list {
		display: flex;
		flex-direction: column;
		gap: 15px;
	}

	.alert-item {
		background: white;
		border: 1px solid #e5e7eb;
		border-left: 4px solid;
		border-radius: 6px;
		padding: 15px;
	}

	.alert-item.critical {
		border-left-color: #dc2626;
		background: #fef2f2;
	}

	.alert-item.warning {
		border-left-color: #f59e0b;
		background: #fefce8;
	}

	.alert-item.info {
		border-left-color: #2563eb;
		background: #eff6ff;
	}

	.alert-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.alert-type {
		font-weight: 600;
		color: #374151;
		text-transform: uppercase;
		font-size: 0.8em;
	}

	.alert-time {
		color: #6b7280;
		font-size: 0.8em;
	}

	.alert-message {
		color: #374151;
		margin-bottom: 5px;
	}

	.alert-details {
		color: #6b7280;
		font-size: 0.9em;
		font-style: italic;
	}

	.config-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		gap: 20px;
	}

	.config-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
	}

	.config-card h4 {
		margin: 0 0 15px 0;
		color: #374151;
	}

	.config-stats {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.config-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.config-label {
		color: #6b7280;
		font-size: 0.9em;
	}

	.config-value {
		font-weight: 600;
		color: #374151;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.security-metrics {
			padding: 15px;
		}

		.header {
			flex-direction: column;
			gap: 15px;
		}

		.header-controls {
			flex-wrap: wrap;
			justify-content: center;
		}

		.health-grid, .metrics-grid, .security-stats-grid, .analytics-grid, .config-grid {
			grid-template-columns: 1fr;
		}

		.alert-header {
			flex-direction: column;
			align-items: flex-start;
			gap: 5px;
		}
	}
</style>