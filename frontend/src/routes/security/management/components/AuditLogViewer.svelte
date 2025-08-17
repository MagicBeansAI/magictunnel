<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte';
	import { writable } from 'svelte/store';
	
	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	// Component state
	let auditLogs = writable([]);
	let loading = true;
	let error = null;
	let selectedLog = null;
	let showFilters = false;
	
	// Filter state
	let filters = {
		event_type: '',
		severity: '',
		user_id: '',
		tool_name: '',
		outcome: '',
		start_date: '',
		end_date: '',
		search_query: ''
	};
	
	let pagination = {
		page: 1,
		limit: 50,
		total: 0,
		totalPages: 0
	};
	
	// Filter options
	const eventTypes = [
		{ value: '', label: 'All Event Types' },
		{ value: 'tool_access', label: 'Tool Access' },
		{ value: 'rule_evaluation', label: 'Rule Evaluation' },
		{ value: 'emergency_lockdown', label: 'Emergency Lockdown' },
		{ value: 'config_change', label: 'Configuration Change' },
		{ value: 'authentication', label: 'Authentication' },
		{ value: 'authorization', label: 'Authorization' },
		{ value: 'pattern_test', label: 'Pattern Testing' }
	];
	
	const severityLevels = [
		{ value: '', label: 'All Severities' },
		{ value: 'low', label: 'Low' },
		{ value: 'medium', label: 'Medium' },
		{ value: 'high', label: 'High' },
		{ value: 'critical', label: 'Critical' }
	];
	
	const outcomes = [
		{ value: '', label: 'All Outcomes' },
		{ value: 'success', label: 'Success' },
		{ value: 'failure', label: 'Failure' },
		{ value: 'blocked', label: 'Blocked' },
		{ value: 'denied', label: 'Denied' },
		{ value: 'error', label: 'Error' }
	];
	
	onMount(async () => {
		await loadAuditLogs();
	});
	
	async function loadAuditLogs() {
		try {
			loading = true;
			
			// Build query parameters
			const params = new URLSearchParams({
				page: pagination.page.toString(),
				limit: pagination.limit.toString()
			});
			
			// Add filters
			Object.entries(filters).forEach(([key, value]) => {
				if (value && value.trim()) {
					params.append(key, value.trim());
				}
			});
			
			const response = await fetch(`/api/security/audit/entries?${params}`);
			if (!response.ok) throw new Error('Failed to load audit logs');
			
			const data = await response.json();
			auditLogs.set(data.entries || []);
			
			// Update pagination
			pagination = {
				...pagination,
				total: data.total || 0,
				totalPages: Math.ceil((data.total || 0) / pagination.limit)
			};
			
		} catch (err) {
			error = err.message;
			console.error('Failed to load audit logs:', err);
		} finally {
			loading = false;
		}
	}
	
	async function applyFilters() {
		pagination.page = 1; // Reset to first page
		await loadAuditLogs();
	}
	
	async function clearFilters() {
		filters = {
			event_type: '',
			severity: '',
			user_id: '',
			tool_name: '',
			outcome: '',
			start_date: '',
			end_date: '',
			search_query: ''
		};
		pagination.page = 1;
		await loadAuditLogs();
	}
	
	async function changePage(newPage) {
		if (newPage >= 1 && newPage <= pagination.totalPages) {
			pagination.page = newPage;
			await loadAuditLogs();
		}
	}
	
	function openLogDetails(log) {
		selectedLog = log;
	}
	
	function closeLogDetails() {
		selectedLog = null;
	}
	
	function toggleFilters() {
		showFilters = !showFilters;
	}
	
	function getSeverityBadgeClass(severity) {
		const classes = {
			low: 'severity-low',
			medium: 'severity-medium',
			high: 'severity-high',
			critical: 'severity-critical'
		};
		return classes[severity] || 'severity-unknown';
	}
	
	function getOutcomeBadgeClass(outcome) {
		const classes = {
			success: 'outcome-success',
			failure: 'outcome-failure',
			blocked: 'outcome-blocked',
			denied: 'outcome-denied',
			error: 'outcome-error'
		};
		return classes[outcome] || 'outcome-unknown';
	}
	
	function formatTimestamp(timestamp) {
		return new Date(timestamp).toLocaleString();
	}
	
	function formatEventType(eventType) {
		return eventType.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
	}
	
	async function exportLogs() {
		try {
			// Build query parameters for export (without pagination)
			const params = new URLSearchParams();
			
			Object.entries(filters).forEach(([key, value]) => {
				if (value && value.trim()) {
					params.append(key, value.trim());
				}
			});
			
			const response = await fetch(`/api/security/audit/export?${params}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					format: 'csv',
					filename: `audit_logs_${new Date().toISOString().split('T')[0]}`
				})
			});
			
			if (!response.ok) throw new Error('Export failed');
			
			// Trigger download
			const blob = await response.blob();
			const url = window.URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `audit_logs_${new Date().toISOString().split('T')[0]}.csv`;
			document.body.appendChild(a);
			a.click();
			window.URL.revokeObjectURL(url);
			document.body.removeChild(a);
			
		} catch (err) {
			console.error('Failed to export logs:', err);
			alert('Failed to export logs: ' + err.message);
		}
	}
	
	// Auto-refresh every 30 seconds
	let refreshInterval;
	onMount(() => {
		refreshInterval = setInterval(async () => {
			if (!loading) {
				await loadAuditLogs();
			}
		}, 30000);
		
		return () => {
			if (refreshInterval) {
				clearInterval(refreshInterval);
			}
		};
	});
</script>

<div class="audit-log-viewer">
	<div class="header">
		<h2>📋 Security Audit Log Viewer</h2>
		<div class="header-actions">
			<button class="btn btn-secondary" on:click={toggleFilters}>
				{showFilters ? '🔍 Hide Filters' : '🔍 Show Filters'}
			</button>
			<button class="btn btn-secondary" on:click={exportLogs}>
				📤 Export Logs
			</button>
			<button class="btn btn-primary" on:click={loadAuditLogs} disabled={loading}>
				{loading ? '🔄 Loading...' : '🔄 Refresh'}
			</button>
		</div>
	</div>

	<!-- Advanced Filters -->
	{#if showFilters}
		<div class="filters-panel">
			<h3>🔍 Advanced Filters</h3>
			
			<div class="filters-grid">
				<div class="filter-group">
					<label>Event Type:</label>
					<select bind:value={filters.event_type}>
						{#each eventTypes as type}
							<option value={type.value}>{type.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>Severity:</label>
					<select bind:value={filters.severity}>
						{#each severityLevels as level}
							<option value={level.value}>{level.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>Outcome:</label>
					<select bind:value={filters.outcome}>
						{#each outcomes as outcome}
							<option value={outcome.value}>{outcome.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>User ID:</label>
					<input type="text" bind:value={filters.user_id} placeholder="Filter by user..." />
				</div>
				
				<div class="filter-group">
					<label>Tool Name:</label>
					<input type="text" bind:value={filters.tool_name} placeholder="Filter by tool..." />
				</div>
				
				<div class="filter-group">
					<label>Search Query:</label>
					<input type="text" bind:value={filters.search_query} placeholder="Search in messages..." />
				</div>
				
				<div class="filter-group">
					<label>Start Date:</label>
					<input type="datetime-local" bind:value={filters.start_date} />
				</div>
				
				<div class="filter-group">
					<label>End Date:</label>
					<input type="datetime-local" bind:value={filters.end_date} />
				</div>
			</div>
			
			<div class="filter-actions">
				<button class="btn btn-primary" on:click={applyFilters}>
					🔍 Apply Filters
				</button>
				<button class="btn btn-secondary" on:click={clearFilters}>
					🗑️ Clear Filters
				</button>
			</div>
		</div>
	{/if}

	<!-- Results Summary -->
	<div class="results-summary">
		<span class="total-count">
			Showing {$auditLogs.length} of {pagination.total} audit events
		</span>
		<span class="page-info">
			Page {pagination.page} of {pagination.totalPages}
		</span>
	</div>

	<!-- Audit Logs Table -->
	{#if loading}
		<div class="loading">Loading audit logs...</div>
	{:else if error}
		<div class="error">Error: {error}</div>
	{:else}
		<div class="logs-table">
			<div class="table-header">
				<div class="col-timestamp">Timestamp</div>
				<div class="col-event">Event Type</div>
				<div class="col-severity">Severity</div>
				<div class="col-user">User</div>
				<div class="col-tool">Tool</div>
				<div class="col-outcome">Outcome</div>
				<div class="col-message">Message</div>
				<div class="col-actions">Actions</div>
			</div>
			
			{#each $auditLogs as log}
				<div class="table-row">
					<div class="col-timestamp">
						<div class="timestamp-main">{formatTimestamp(log.timestamp).split(',')[0]}</div>
						<div class="timestamp-time">{formatTimestamp(log.timestamp).split(',')[1]}</div>
					</div>
					
					<div class="col-event">
						<span class="event-badge">{formatEventType(log.event_type)}</span>
					</div>
					
					<div class="col-severity">
						<span class="severity-badge {getSeverityBadgeClass(log.severity)}">
							{log.severity || 'N/A'}
						</span>
					</div>
					
					<div class="col-user">
						{log.user?.user_id || 'System'}
					</div>
					
					<div class="col-tool">
						{log.tool?.name || '-'}
					</div>
					
					<div class="col-outcome">
						<span class="outcome-badge {getOutcomeBadgeClass(log.outcome)}">
							{log.outcome || 'N/A'}
						</span>
					</div>
					
					<div class="col-message">
						<div class="message-text">{log.message}</div>
						{#if log.details && Object.keys(log.details).length > 0}
							<div class="message-details">+{Object.keys(log.details).length} details</div>
						{/if}
					</div>
					
					<div class="col-actions">
						<button class="btn-icon" on:click={() => openLogDetails(log)} title="View Details">
							👁️
						</button>
					</div>
				</div>
			{/each}
			
			{#if $auditLogs.length === 0}
				<div class="no-results">
					No audit logs match your current filters
				</div>
			{/if}
		</div>
	{/if}

	<!-- Pagination -->
	{#if pagination.totalPages > 1}
		<div class="pagination">
			<button 
				class="btn btn-secondary" 
				on:click={() => changePage(pagination.page - 1)}
				disabled={pagination.page <= 1}
			>
				← Previous
			</button>
			
			<div class="page-numbers">
				{#each Array(Math.min(5, pagination.totalPages)) as _, i}
					{@const pageNum = Math.max(1, pagination.page - 2) + i}
					{#if pageNum <= pagination.totalPages}
						<button 
							class="page-btn {pageNum === pagination.page ? 'active' : ''}"
							on:click={() => changePage(pageNum)}
						>
							{pageNum}
						</button>
					{/if}
				{/each}
			</div>
			
			<button 
				class="btn btn-secondary" 
				on:click={() => changePage(pagination.page + 1)}
				disabled={pagination.page >= pagination.totalPages}
			>
				Next →
			</button>
		</div>
	{/if}
</div>

<!-- Log Details Modal -->
{#if selectedLog}
	<div class="modal-backdrop" on:click={closeLogDetails}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>Audit Log Details</h3>
				<button class="close-btn" on:click={closeLogDetails}>✕</button>
			</div>
			
			<div class="modal-content">
				<div class="detail-grid">
					<div class="detail-item">
						<label>Event ID:</label>
						<span>{selectedLog.id}</span>
					</div>
					<div class="detail-item">
						<label>Timestamp:</label>
						<span>{formatTimestamp(selectedLog.timestamp)}</span>
					</div>
					<div class="detail-item">
						<label>Event Type:</label>
						<span class="event-badge">{formatEventType(selectedLog.event_type)}</span>
					</div>
					<div class="detail-item">
						<label>Severity:</label>
						<span class="severity-badge {getSeverityBadgeClass(selectedLog.severity)}">
							{selectedLog.severity || 'N/A'}
						</span>
					</div>
					<div class="detail-item">
						<label>Outcome:</label>
						<span class="outcome-badge {getOutcomeBadgeClass(selectedLog.outcome)}">
							{selectedLog.outcome || 'N/A'}
						</span>
					</div>
					<div class="detail-item">
						<label>Source IP:</label>
						<span>{selectedLog.source_ip || 'N/A'}</span>
					</div>
					
					{#if selectedLog.user}
						<div class="detail-section">
							<h4>👤 User Information</h4>
							<div class="nested-details">
								<div class="detail-item">
									<label>User ID:</label>
									<span>{selectedLog.user.user_id}</span>
								</div>
								{#if selectedLog.user.email}
									<div class="detail-item">
										<label>Email:</label>
										<span>{selectedLog.user.email}</span>
									</div>
								{/if}
								{#if selectedLog.user.roles && selectedLog.user.roles.length > 0}
									<div class="detail-item">
										<label>Roles:</label>
										<span>{selectedLog.user.roles.join(', ')}</span>
									</div>
								{/if}
							</div>
						</div>
					{/if}
					
					{#if selectedLog.tool}
						<div class="detail-section">
							<h4>🔧 Tool Information</h4>
							<div class="nested-details">
								<div class="detail-item">
									<label>Tool Name:</label>
									<span>{selectedLog.tool.name}</span>
								</div>
								{#if selectedLog.tool.category}
									<div class="detail-item">
										<label>Category:</label>
										<span>{selectedLog.tool.category}</span>
									</div>
								{/if}
								{#if selectedLog.tool.parameters}
									<div class="detail-item full-width">
										<label>Parameters:</label>
										<pre>{JSON.stringify(selectedLog.tool.parameters, null, 2)}</pre>
									</div>
								{/if}
							</div>
						</div>
					{/if}
					
					<div class="detail-item full-width">
						<label>Message:</label>
						<span>{selectedLog.message}</span>
					</div>
					
					{#if selectedLog.details && Object.keys(selectedLog.details).length > 0}
						<div class="detail-item full-width">
							<label>Additional Details:</label>
							<pre>{JSON.stringify(selectedLog.details, null, 2)}</pre>
						</div>
					{/if}
					
					{#if selectedLog.request_id}
						<div class="detail-item">
							<label>Request ID:</label>
							<span class="monospace">{selectedLog.request_id}</span>
						</div>
					{/if}
					
					{#if selectedLog.session_id}
						<div class="detail-item">
							<label>Session ID:</label>
							<span class="monospace">{selectedLog.session_id}</span>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.audit-log-viewer {
		padding: 20px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 20px;
	}

	.header h2 {
		margin: 0;
		color: #1f2937;
	}

	.header-actions {
		display: flex;
		gap: 10px;
	}

	.btn {
		padding: 8px 16px;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.btn-primary {
		background: #2563eb;
		color: white;
	}

	.btn-primary:hover {
		background: #1d4ed8;
	}

	.btn-secondary {
		background: #6b7280;
		color: white;
	}

	.btn-secondary:hover {
		background: #4b5563;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.filters-panel {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
		margin-bottom: 20px;
	}

	.filters-panel h3 {
		margin: 0 0 15px 0;
		color: #374151;
	}

	.filters-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 15px;
		margin-bottom: 15px;
	}

	.filter-group {
		display: flex;
		flex-direction: column;
	}

	.filter-group label {
		font-weight: 600;
		color: #374151;
		margin-bottom: 5px;
		font-size: 0.9em;
	}

	.filter-group input, .filter-group select {
		padding: 8px 12px;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 0.9em;
	}

	.filter-actions {
		display: flex;
		gap: 10px;
	}

	.results-summary {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px 0;
		margin-bottom: 15px;
		border-bottom: 1px solid #e5e7eb;
		color: #6b7280;
		font-size: 0.9em;
	}

	.logs-table {
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		overflow: hidden;
		background: white;
		margin-bottom: 20px;
	}

	.table-header {
		display: grid;
		grid-template-columns: 140px 120px 90px 120px 120px 90px 2fr 60px;
		background: #f9fafb;
		border-bottom: 1px solid #e5e7eb;
		font-weight: 600;
		color: #374151;
	}

	.table-header > div {
		padding: 12px 8px;
		border-right: 1px solid #e5e7eb;
	}

	.table-header > div:last-child {
		border-right: none;
	}

	.table-row {
		display: grid;
		grid-template-columns: 140px 120px 90px 120px 120px 90px 2fr 60px;
		border-bottom: 1px solid #f3f4f6;
		transition: background-color 0.2s;
	}

	.table-row:hover {
		background: #f9fafb;
	}

	.table-row > div {
		padding: 12px 8px;
		display: flex;
		align-items: center;
		border-right: 1px solid #f3f4f6;
		min-height: 50px;
	}

	.table-row > div:last-child {
		border-right: none;
	}

	.timestamp-main {
		font-weight: 600;
		color: #374151;
	}

	.timestamp-time {
		font-size: 0.8em;
		color: #6b7280;
	}

	.col-timestamp {
		flex-direction: column !important;
		align-items: flex-start !important;
	}

	.event-badge, .severity-badge, .outcome-badge {
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 0.8em;
		font-weight: 600;
		text-transform: uppercase;
	}

	.event-badge {
		background: #eff6ff;
		color: #2563eb;
	}

	.severity-low {
		background: #f0fdf4;
		color: #16a34a;
	}

	.severity-medium {
		background: #fefce8;
		color: #ca8a04;
	}

	.severity-high {
		background: #fef3c7;
		color: #f59e0b;
	}

	.severity-critical {
		background: #fef2f2;
		color: #dc2626;
	}

	.outcome-success {
		background: #f0fdf4;
		color: #16a34a;
	}

	.outcome-failure, .outcome-error {
		background: #fef2f2;
		color: #dc2626;
	}

	.outcome-blocked, .outcome-denied {
		background: #fef3c7;
		color: #f59e0b;
	}

	.message-text {
		font-size: 0.9em;
		color: #374151;
		line-height: 1.4;
	}

	.message-details {
		font-size: 0.8em;
		color: #6b7280;
		margin-top: 4px;
		font-style: italic;
	}

	.col-message {
		flex-direction: column !important;
		align-items: flex-start !important;
	}

	.btn-icon {
		background: none;
		border: none;
		cursor: pointer;
		padding: 4px;
		border-radius: 4px;
		transition: background-color 0.2s;
	}

	.btn-icon:hover {
		background: #f3f4f6;
	}

	.loading, .error, .no-results {
		text-align: center;
		padding: 40px 20px;
		color: #6b7280;
	}

	.pagination {
		display: flex;
		justify-content: center;
		align-items: center;
		gap: 10px;
		margin-top: 20px;
	}

	.page-numbers {
		display: flex;
		gap: 5px;
	}

	.page-btn {
		padding: 8px 12px;
		border: 1px solid #d1d5db;
		background: white;
		cursor: pointer;
		border-radius: 4px;
		transition: all 0.2s;
	}

	.page-btn:hover {
		background: #f3f4f6;
	}

	.page-btn.active {
		background: #2563eb;
		color: white;
		border-color: #2563eb;
	}

	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 12px;
		max-width: 800px;
		width: 90%;
		max-height: 80vh;
		overflow: auto;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 20px;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h3 {
		margin: 0;
		color: #1f2937;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 18px;
		cursor: pointer;
		padding: 5px;
		border-radius: 4px;
	}

	.close-btn:hover {
		background: #f3f4f6;
	}

	.modal-content {
		padding: 20px;
	}

	.detail-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 15px;
	}

	.detail-item {
		display: flex;
		flex-direction: column;
	}

	.detail-item.full-width {
		grid-column: 1 / -1;
	}

	.detail-item label {
		font-weight: 600;
		color: #374151;
		margin-bottom: 5px;
	}

	.detail-item span, .detail-item pre {
		color: #6b7280;
	}

	.detail-item pre {
		background: #f9fafb;
		padding: 10px;
		border-radius: 4px;
		font-size: 0.9em;
		overflow-x: auto;
	}

	.detail-section {
		grid-column: 1 / -1;
		border-top: 1px solid #e5e7eb;
		padding-top: 15px;
		margin-top: 10px;
	}

	.detail-section h4 {
		margin: 0 0 10px 0;
		color: #374151;
		font-size: 1em;
	}

	.nested-details {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
		margin-left: 15px;
	}

	.monospace {
		font-family: monospace;
		background: #f3f4f6;
		padding: 2px 6px;
		border-radius: 3px;
		font-size: 0.9em;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.audit-log-viewer {
			padding: 15px;
		}

		.header {
			flex-direction: column;
			gap: 10px;
		}

		.header-actions {
			flex-wrap: wrap;
		}

		.filters-grid {
			grid-template-columns: 1fr;
		}

		.table-header, .table-row {
			grid-template-columns: 1fr;
		}

		.table-header > div, .table-row > div {
			border-right: none;
			border-bottom: 1px solid #f3f4f6;
		}

		.table-header > div:last-child, .table-row > div:last-child {
			border-bottom: none;
		}

		.detail-grid, .nested-details {
			grid-template-columns: 1fr;
		}

		.pagination {
			flex-wrap: wrap;
		}
	}
</style>