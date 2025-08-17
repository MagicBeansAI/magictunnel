<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte';
	import { writable } from 'svelte/store';
	
	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	// Component state
	let changes = writable([]);
	let loading = true;
	let error = null;
	let selectedChange = null;
	let showFilters = false;
	
	// Filter state
	let filters = {
		change_type: '',
		operation: '',
		user_id: '',
		target_type: '',
		start_date: '',
		end_date: '',
		search_query: ''
	};
	
	let pagination = {
		page: 1,
		limit: 20,
		total: 0,
		totalPages: 0
	};
	
	let statistics = null;
	
	// Filter options
	const changeTypes = [
		{ value: '', label: 'All Change Types' },
		{ value: 'allowlist_rule', label: 'Allowlist Rules' },
		{ value: 'pattern_rule', label: 'Pattern Rules' },
		{ value: 'emergency_lockdown', label: 'Emergency Lockdown' },
		{ value: 'security_config', label: 'Security Configuration' },
		{ value: 'user_permissions', label: 'User Permissions' },
		{ value: 'audit_settings', label: 'Audit Settings' }
	];
	
	const operations = [
		{ value: '', label: 'All Operations' },
		{ value: 'create', label: 'Create' },
		{ value: 'update', label: 'Update' },
		{ value: 'delete', label: 'Delete' },
		{ value: 'enable', label: 'Enable' },
		{ value: 'disable', label: 'Disable' }
	];
	
	const targetTypes = [
		{ value: '', label: 'All Targets' },
		{ value: 'tool_rule', label: 'Tool Rules' },
		{ value: 'capability_pattern', label: 'Capability Patterns' },
		{ value: 'global_pattern', label: 'Global Patterns' },
		{ value: 'emergency_system', label: 'Emergency System' },
		{ value: 'configuration_file', label: 'Configuration Files' }
	];
	
	onMount(async () => {
		await loadChanges();
		await loadStatistics();
	});
	
	async function loadChanges() {
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
			
			const response = await fetch(`/api/security/changes?${params}`);
			if (!response.ok) throw new Error('Failed to load changes');
			
			const data = await response.json();
			changes.set(data.data.changes || []);
			
			// Update pagination
			pagination = {
				...pagination,
				total: data.data.total || 0,
				totalPages: data.data.total_pages || 0
			};
			
		} catch (err) {
			error = err.message;
			console.error('Failed to load changes:', err);
		} finally {
			loading = false;
		}
	}
	
	async function loadStatistics() {
		try {
			const response = await fetch('/api/security/changes/statistics');
			if (response.ok) {
				const data = await response.json();
				statistics = data.data;
			}
		} catch (err) {
			console.error('Failed to load statistics:', err);
		}
	}
	
	async function applyFilters() {
		pagination.page = 1; // Reset to first page
		await loadChanges();
	}
	
	async function clearFilters() {
		filters = {
			change_type: '',
			operation: '',
			user_id: '',
			target_type: '',
			start_date: '',
			end_date: '',
			search_query: ''
		};
		pagination.page = 1;
		await loadChanges();
	}
	
	async function changePage(newPage) {
		if (newPage >= 1 && newPage <= pagination.totalPages) {
			pagination.page = newPage;
			await loadChanges();
		}
	}
	
	function openChangeDetails(change) {
		selectedChange = change;
	}
	
	function closeChangeDetails() {
		selectedChange = null;
	}
	
	function toggleFilters() {
		showFilters = !showFilters;
	}
	
	function getOperationBadgeClass(operation) {
		const classes = {
			create: 'operation-create',
			update: 'operation-update',
			delete: 'operation-delete',
			enable: 'operation-enable',
			disable: 'operation-disable'
		};
		return classes[operation] || 'operation-unknown';
	}
	
	function getImpactBadgeClass(severity) {
		const classes = {
			low: 'impact-low',
			medium: 'impact-medium',
			high: 'impact-high',
			critical: 'impact-critical'
		};
		return classes[severity] || 'impact-unknown';
	}
	
	function formatTimestamp(timestamp) {
		return new Date(timestamp).toLocaleString();
	}
	
	function formatChangeType(changeType) {
		return changeType.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase());
	}
	
	function formatDuration(startTime, endTime) {
		if (!startTime || !endTime) return 'N/A';
		const start = new Date(startTime);
		const end = new Date(endTime);
		const diffMs = end - start;
		return `${Math.round(diffMs)}ms`;
	}
	
	async function exportChanges() {
		try {
			// Build query parameters for export (without pagination)
			const params = new URLSearchParams();
			
			Object.entries(filters).forEach(([key, value]) => {
				if (value && value.trim()) {
					params.append(key, value.trim());
				}
			});
			
			const response = await fetch(`/api/security/changes/export?${params}`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					format: 'csv',
					filename: `security_changes_${new Date().toISOString().split('T')[0]}`
				})
			});
			
			if (!response.ok) throw new Error('Export failed');
			
			// Trigger download
			const blob = await response.blob();
			const url = window.URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `security_changes_${new Date().toISOString().split('T')[0]}.csv`;
			document.body.appendChild(a);
			a.click();
			window.URL.revokeObjectURL(url);
			document.body.removeChild(a);
			
		} catch (err) {
			console.error('Failed to export changes:', err);
			alert('Failed to export changes: ' + err.message);
		}
	}
	
	// Auto-refresh every 30 seconds
	let refreshInterval;
	onMount(() => {
		refreshInterval = setInterval(async () => {
			if (!loading) {
				await loadChanges();
				await loadStatistics();
			}
		}, 30000);
		
		return () => {
			if (refreshInterval) {
				clearInterval(refreshInterval);
			}
		};
	});
</script>

<div class="change-tracking">
	<div class="header">
		<h2>📊 Configuration Change Tracking</h2>
		<div class="header-actions">
			<button class="btn btn-secondary" on:click={toggleFilters}>
				{showFilters ? '🔍 Hide Filters' : '🔍 Show Filters'}
			</button>
			<button class="btn btn-secondary" on:click={exportChanges}>
				📤 Export Changes
			</button>
			<button class="btn btn-primary" on:click={loadChanges} disabled={loading}>
				{loading ? '🔄 Loading...' : '🔄 Refresh'}
			</button>
		</div>
	</div>

	<!-- Statistics Cards -->
	{#if statistics}
		<div class="statistics-grid">
			<div class="stat-card">
				<h4>📈 Total Changes</h4>
				<div class="stat-value">{statistics.total_changes || 0}</div>
				<div class="stat-label">All time</div>
			</div>
			
			<div class="stat-card">
				<h4>🕐 Recent Changes</h4>
				<div class="stat-value">{statistics.recent_changes || 0}</div>
				<div class="stat-label">Last 24 hours</div>
			</div>
			
			<div class="stat-card">
				<h4>👥 Active Users</h4>
				<div class="stat-value">{statistics.active_users || 0}</div>
				<div class="stat-label">Last 7 days</div>
			</div>
			
			<div class="stat-card">
				<h4>⚠️ High Impact</h4>
				<div class="stat-value">{statistics.high_impact_changes || 0}</div>
				<div class="stat-label">Critical changes</div>
			</div>
		</div>
	{/if}

	<!-- Advanced Filters -->
	{#if showFilters}
		<div class="filters-panel">
			<h3>🔍 Advanced Filters</h3>
			
			<div class="filters-grid">
				<div class="filter-group">
					<label>Change Type:</label>
					<select bind:value={filters.change_type}>
						{#each changeTypes as type}
							<option value={type.value}>{type.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>Operation:</label>
					<select bind:value={filters.operation}>
						{#each operations as operation}
							<option value={operation.value}>{operation.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>Target Type:</label>
					<select bind:value={filters.target_type}>
						{#each targetTypes as target}
							<option value={target.value}>{target.label}</option>
						{/each}
					</select>
				</div>
				
				<div class="filter-group">
					<label>User ID:</label>
					<input type="text" bind:value={filters.user_id} placeholder="Filter by user..." />
				</div>
				
				<div class="filter-group">
					<label>Search Query:</label>
					<input type="text" bind:value={filters.search_query} placeholder="Search in descriptions..." />
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
			Showing {$changes.length} of {pagination.total} configuration changes
		</span>
		<span class="page-info">
			Page {pagination.page} of {pagination.totalPages}
		</span>
	</div>

	<!-- Changes Table -->
	{#if loading}
		<div class="loading">Loading configuration changes...</div>
	{:else if error}
		<div class="error">Error: {error}</div>
	{:else}
		<div class="changes-table">
			<div class="table-header">
				<div class="col-timestamp">Timestamp</div>
				<div class="col-type">Change Type</div>
				<div class="col-operation">Operation</div>
				<div class="col-user">User</div>
				<div class="col-target">Target</div>
				<div class="col-impact">Impact</div>
				<div class="col-description">Description</div>
				<div class="col-actions">Actions</div>
			</div>
			
			{#each $changes as change}
				<div class="table-row">
					<div class="col-timestamp">
						<div class="timestamp-main">{formatTimestamp(change.timestamp).split(',')[0]}</div>
						<div class="timestamp-time">{formatTimestamp(change.timestamp).split(',')[1]}</div>
					</div>
					
					<div class="col-type">
						<span class="type-badge">{formatChangeType(change.change_type)}</span>
					</div>
					
					<div class="col-operation">
						<span class="operation-badge {getOperationBadgeClass(change.operation)}">
							{change.operation}
						</span>
					</div>
					
					<div class="col-user">
						<div class="user-info">
							<div class="user-id">{change.user.user_id}</div>
							{#if change.user.authentication_method}
								<div class="auth-method">{change.user.authentication_method}</div>
							{/if}
						</div>
					</div>
					
					<div class="col-target">
						<div class="target-info">
							<div class="target-type">{change.target.target_type}</div>
							<div class="target-id">{change.target.target_id}</div>
						</div>
					</div>
					
					<div class="col-impact">
						<span class="impact-badge {getImpactBadgeClass(change.impact.severity)}">
							{change.impact.severity}
						</span>
						<div class="affected-tools">
							{change.impact.affected_tools_count} tools
						</div>
					</div>
					
					<div class="col-description">
						<div class="description-text">{change.description}</div>
						{#if change.diff && Object.keys(change.diff).length > 0}
							<div class="diff-indicator">+{Object.keys(change.diff).length} changes</div>
						{/if}
					</div>
					
					<div class="col-actions">
						<button class="btn-icon" on:click={() => openChangeDetails(change)} title="View Details">
							👁️
						</button>
					</div>
				</div>
			{/each}
			
			{#if $changes.length === 0}
				<div class="no-results">
					No configuration changes match your current filters
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

<!-- Change Details Modal -->
{#if selectedChange}
	<div class="modal-backdrop" on:click={closeChangeDetails}>
		<div class="modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>Configuration Change Details</h3>
				<button class="close-btn" on:click={closeChangeDetails}>✕</button>
			</div>
			
			<div class="modal-content">
				<div class="detail-grid">
					<div class="detail-item">
						<label>Change ID:</label>
						<span class="monospace">{selectedChange.id}</span>
					</div>
					<div class="detail-item">
						<label>Timestamp:</label>
						<span>{formatTimestamp(selectedChange.timestamp)}</span>
					</div>
					<div class="detail-item">
						<label>Change Type:</label>
						<span class="type-badge">{formatChangeType(selectedChange.change_type)}</span>
					</div>
					<div class="detail-item">
						<label>Operation:</label>
						<span class="operation-badge {getOperationBadgeClass(selectedChange.operation)}">
							{selectedChange.operation}
						</span>
					</div>
					
					<!-- User Information -->
					<div class="detail-section">
						<h4>👤 User Information</h4>
						<div class="nested-details">
							<div class="detail-item">
								<label>User ID:</label>
								<span>{selectedChange.user.user_id}</span>
							</div>
							<div class="detail-item">
								<label>Authentication:</label>
								<span>{selectedChange.user.authentication_method || 'N/A'}</span>
							</div>
							{#if selectedChange.user.client_info}
								<div class="detail-item">
									<label>Client IP:</label>
									<span>{selectedChange.user.client_info.ip_address || 'N/A'}</span>
								</div>
								<div class="detail-item">
									<label>User Agent:</label>
									<span>{selectedChange.user.client_info.user_agent || 'N/A'}</span>
								</div>
							{/if}
						</div>
					</div>
					
					<!-- Target Information -->
					<div class="detail-section">
						<h4>🎯 Target Information</h4>
						<div class="nested-details">
							<div class="detail-item">
								<label>Target Type:</label>
								<span>{selectedChange.target.target_type}</span>
							</div>
							<div class="detail-item">
								<label>Target ID:</label>
								<span>{selectedChange.target.target_id}</span>
							</div>
							{#if selectedChange.target.file_path}
								<div class="detail-item full-width">
									<label>File Path:</label>
									<span class="monospace">{selectedChange.target.file_path}</span>
								</div>
							{/if}
						</div>
					</div>
					
					<!-- Impact Analysis -->
					<div class="detail-section">
						<h4>⚡ Impact Analysis</h4>
						<div class="nested-details">
							<div class="detail-item">
								<label>Severity:</label>
								<span class="impact-badge {getImpactBadgeClass(selectedChange.impact.severity)}">
									{selectedChange.impact.severity}
								</span>
							</div>
							<div class="detail-item">
								<label>Affected Tools:</label>
								<span>{selectedChange.impact.affected_tools_count}</span>
							</div>
							{#if selectedChange.impact.affected_tools && selectedChange.impact.affected_tools.length > 0}
								<div class="detail-item full-width">
									<label>Tool List:</label>
									<div class="tool-list">
										{#each selectedChange.impact.affected_tools as tool}
											<span class="tool-tag">{tool}</span>
										{/each}
									</div>
								</div>
							{/if}
							{#if selectedChange.impact.description}
								<div class="detail-item full-width">
									<label>Impact Description:</label>
									<span>{selectedChange.impact.description}</span>
								</div>
							{/if}
						</div>
					</div>
					
					<div class="detail-item full-width">
						<label>Description:</label>
						<span>{selectedChange.description}</span>
					</div>
					
					<!-- Change Diff -->
					{#if selectedChange.diff && Object.keys(selectedChange.diff).length > 0}
						<div class="detail-section">
							<h4>🔄 Configuration Changes</h4>
							<div class="diff-container">
								{#each Object.entries(selectedChange.diff) as [field, changes]}
									<div class="diff-field">
										<h5>{field}</h5>
										<div class="diff-content">
											{#if changes.before !== undefined}
												<div class="diff-before">
													<strong>Before:</strong>
													<pre>{JSON.stringify(changes.before, null, 2)}</pre>
												</div>
											{/if}
											{#if changes.after !== undefined}
												<div class="diff-after">
													<strong>After:</strong>
													<pre>{JSON.stringify(changes.after, null, 2)}</pre>
												</div>
											{/if}
										</div>
									</div>
								{/each}
							</div>
						</div>
					{/if}
					
					<!-- Validation Results -->
					{#if selectedChange.validation}
						<div class="detail-section">
							<h4>✅ Validation Results</h4>
							<div class="validation-results">
								<div class="validation-item">
									<label>Syntax Valid:</label>
									<span class="validation-badge {selectedChange.validation.syntax_valid ? 'valid' : 'invalid'}">
										{selectedChange.validation.syntax_valid ? '✅ Valid' : '❌ Invalid'}
									</span>
								</div>
								<div class="validation-item">
									<label>Semantic Valid:</label>
									<span class="validation-badge {selectedChange.validation.semantic_valid ? 'valid' : 'invalid'}">
										{selectedChange.validation.semantic_valid ? '✅ Valid' : '❌ Invalid'}
									</span>
								</div>
								{#if selectedChange.validation.warnings && selectedChange.validation.warnings.length > 0}
									<div class="validation-item full-width">
										<label>Warnings:</label>
										<ul>
											{#each selectedChange.validation.warnings as warning}
												<li>{warning}</li>
											{/each}
										</ul>
									</div>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.change-tracking {
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

	.statistics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 20px;
		margin-bottom: 30px;
	}

	.stat-card {
		background: white;
		padding: 20px;
		border-radius: 8px;
		border: 1px solid #e5e7eb;
		text-align: center;
	}

	.stat-card h4 {
		margin: 0 0 10px 0;
		color: #6b7280;
		font-size: 0.9em;
	}

	.stat-value {
		font-size: 2em;
		font-weight: bold;
		color: #111827;
		margin-bottom: 5px;
	}

	.stat-label {
		color: #9ca3af;
		font-size: 0.8em;
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

	.changes-table {
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		overflow: hidden;
		background: white;
		margin-bottom: 20px;
	}

	.table-header {
		display: grid;
		grid-template-columns: 140px 120px 100px 140px 120px 90px 2fr 60px;
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
		grid-template-columns: 140px 120px 100px 140px 120px 90px 2fr 60px;
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
		min-height: 60px;
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

	.type-badge, .operation-badge, .impact-badge {
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 0.8em;
		font-weight: 600;
		text-transform: uppercase;
	}

	.type-badge {
		background: #eff6ff;
		color: #2563eb;
	}

	.operation-create {
		background: #f0fdf4;
		color: #16a34a;
	}

	.operation-update {
		background: #fefce8;
		color: #ca8a04;
	}

	.operation-delete {
		background: #fef2f2;
		color: #dc2626;
	}

	.operation-enable {
		background: #f0f9ff;
		color: #0ea5e9;
	}

	.operation-disable {
		background: #fef3c7;
		color: #f59e0b;
	}

	.impact-low {
		background: #f0fdf4;
		color: #16a34a;
	}

	.impact-medium {
		background: #fefce8;
		color: #ca8a04;
	}

	.impact-high {
		background: #fef3c7;
		color: #f59e0b;
	}

	.impact-critical {
		background: #fef2f2;
		color: #dc2626;
	}

	.user-info, .target-info {
		flex-direction: column !important;
		align-items: flex-start !important;
	}

	.user-id, .target-type {
		font-weight: 600;
		color: #374151;
	}

	.auth-method, .target-id {
		font-size: 0.8em;
		color: #6b7280;
		margin-top: 2px;
	}

	.affected-tools {
		font-size: 0.8em;
		color: #6b7280;
		margin-top: 4px;
	}

	.col-impact {
		flex-direction: column !important;
		align-items: flex-start !important;
	}

	.description-text {
		font-size: 0.9em;
		color: #374151;
		line-height: 1.4;
	}

	.diff-indicator {
		font-size: 0.8em;
		color: #6b7280;
		margin-top: 4px;
		font-style: italic;
	}

	.col-description {
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
		max-width: 900px;
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

	.tool-list {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: 5px;
	}

	.tool-tag {
		background: #eff6ff;
		color: #2563eb;
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 0.8em;
		font-family: monospace;
	}

	.diff-container {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		padding: 15px;
		margin-top: 10px;
	}

	.diff-field {
		margin-bottom: 15px;
	}

	.diff-field h5 {
		margin: 0 0 8px 0;
		color: #374151;
		font-size: 0.9em;
	}

	.diff-content {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 10px;
	}

	.diff-before, .diff-after {
		background: white;
		border-radius: 4px;
		padding: 10px;
	}

	.diff-before {
		border-left: 3px solid #ef4444;
	}

	.diff-after {
		border-left: 3px solid #10b981;
	}

	.diff-before pre, .diff-after pre {
		margin: 5px 0 0 0;
		font-size: 0.8em;
		overflow-x: auto;
	}

	.validation-results {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		padding: 15px;
		margin-top: 10px;
	}

	.validation-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 10px;
	}

	.validation-badge {
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 0.8em;
		font-weight: 600;
	}

	.validation-badge.valid {
		background: #f0fdf4;
		color: #16a34a;
	}

	.validation-badge.invalid {
		background: #fef2f2;
		color: #dc2626;
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
		.change-tracking {
			padding: 15px;
		}

		.header {
			flex-direction: column;
			gap: 10px;
		}

		.header-actions {
			flex-wrap: wrap;
		}

		.statistics-grid {
			grid-template-columns: 1fr;
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

		.diff-content {
			grid-template-columns: 1fr;
		}

		.pagination {
			flex-wrap: wrap;
		}
	}
</style>