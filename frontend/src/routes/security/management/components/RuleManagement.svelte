<script lang="ts">
	import { onMount, createEventDispatcher } from 'svelte';
	import { securityApi } from '$lib/api/security';
	import AllowlistRuleCard from '$lib/components/security/AllowlistRuleCard.svelte';
	import AllowlistRuleEditor from '$lib/components/security/AllowlistRuleEditor.svelte';
	import AllowlistRuleTester from '$lib/components/security/AllowlistRuleTester.svelte';
	import type { AllowlistRule, CreateAllowlistRule } from '$lib/types/security';

	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	// State management
	let rules: AllowlistRule[] = [];
	let loading = true;
	let error = '';
	let searchQuery = '';
	let filterType: 'all' | 'tool' | 'resource' | 'global' = 'all';
	let filterAction: 'all' | 'allow' | 'deny' = 'all';
	let sortBy: 'name' | 'priority' | 'created' | 'modified' = 'priority';
	let sortOrder: 'asc' | 'desc' = 'desc';

	// Modal states
	let showRuleEditor = false;
	let showRuleTester = false;
	let editingRule: AllowlistRule | null = null;

	// Selection and bulk operations
	let selectedRules = new Set<string>();
	let showBulkActions = false;
	let bulkOperationInProgress = false;

	// Pagination
	let currentPage = 1;
	let itemsPerPage = 20;

	// Statistics
	$: ruleStats = calculateRuleStats(rules);
	$: filteredRules = filterAndSortRules(rules, searchQuery, filterType, filterAction, sortBy, sortOrder);
	$: paginatedRules = paginateRules(filteredRules, currentPage, itemsPerPage);
	$: totalPages = Math.ceil(filteredRules.length / itemsPerPage);

	function calculateRuleStats(rules: AllowlistRule[]) {
		const stats = {
			total: rules.length,
			active: rules.filter(r => r.active).length,
			inactive: rules.filter(r => !r.active).length,
			byType: {
				tool: rules.filter(r => r.type === 'tool').length,
				resource: rules.filter(r => r.type === 'resource').length,
				global: rules.filter(r => r.type === 'global').length,
			},
			byAction: {
				allow: rules.filter(r => r.action === 'allow').length,
				deny: rules.filter(r => r.action === 'deny').length,
			}
		};
		return stats;
	}

	function filterAndSortRules(
		rules: AllowlistRule[],
		query: string,
		type: string,
		action: string,
		sortBy: string,
		sortOrder: string
	): AllowlistRule[] {
		let filtered = [...rules];

		// Apply search filter
		if (query.trim()) {
			const lowerQuery = query.toLowerCase();
			filtered = filtered.filter(rule =>
				rule.name.toLowerCase().includes(lowerQuery) ||
				rule.pattern.toLowerCase().includes(lowerQuery)
			);
		}

		// Apply type filter
		if (type !== 'all') {
			filtered = filtered.filter(rule => rule.type === type);
		}

		// Apply action filter
		if (action !== 'all') {
			filtered = filtered.filter(rule => rule.action === action);
		}

		// Apply sorting
		filtered.sort((a, b) => {
			let comparison = 0;
			
			switch (sortBy) {
				case 'name':
					comparison = a.name.localeCompare(b.name);
					break;
				case 'priority':
					comparison = a.priority - b.priority;
					break;
				case 'created':
					comparison = new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
					break;
				case 'modified':
					comparison = new Date(a.modifiedAt).getTime() - new Date(b.modifiedAt).getTime();
					break;
			}

			return sortOrder === 'asc' ? comparison : -comparison;
		});

		return filtered;
	}

	function paginateRules(rules: AllowlistRule[], page: number, itemsPerPage: number): AllowlistRule[] {
		const startIndex = (page - 1) * itemsPerPage;
		return rules.slice(startIndex, startIndex + itemsPerPage);
	}

	// Load allowlist rules
	async function loadRules() {
		try {
			loading = true;
			error = '';
			
			// Use the working API endpoint directly
			const response = await fetch('/api/security/allowlist/rules');
			if (!response.ok) {
				throw new Error(`API error: ${response.status} ${response.statusText}`);
			}
			
			const data = await response.json();
			
			// The API returns { rules: [...], total_rules: N, ... }
			const backendRules = data.rules || [];
			
			// Map backend fields to frontend fields for compatibility
			rules = backendRules.map(rule => ({
				...rule,
				// Map backend fields to frontend expected fields
				active: rule.enabled ?? true,
				type: rule.rule_type || 'tool',
				createdAt: rule.created_at ? new Date(rule.created_at) : new Date(),
				modifiedAt: rule.updated_at ? new Date(rule.updated_at) : new Date(),
				conditions: rule.conditions || []
			}));
			
			// Update security state with rule statistics
			if (securityState) {
				securityState.update(state => ({
					...state,
					ruleStats: {
						totalRules: data.total_rules || rules.length,
						activeRules: rules.filter(r => r.active !== false).length,
						conflicts: 0 // We'll add conflict detection later
					}
				}));
			}
		} catch (err) {
			console.error('Failed to load allowlist rules:', err);
			error = `Failed to load rules: ${err.message}`;
		} finally {
			loading = false;
		}
	}

	// Rule operations
	async function deleteRule(ruleId: string) {
		if (!confirm('Are you sure you want to delete this rule? This action cannot be undone.')) {
			return;
		}

		try {
			const response = await fetch(`/api/security/allowlist/rules/${ruleId}`, {
				method: 'DELETE'
			});
			
			if (!response.ok) {
				throw new Error(`API error: ${response.status} ${response.statusText}`);
			}
			
			await loadRules();
			dispatch('refresh');
		} catch (err) {
			alert(`Failed to delete rule: ${err.message}`);
		}
	}

	async function toggleRuleStatus(rule: AllowlistRule) {
		try {
			// Map frontend fields to backend fields
			const updateData = {
				enabled: !(rule.active ?? rule.enabled ?? true)
			};
			
			const response = await fetch(`/api/security/allowlist/rules/${rule.id}`, {
				method: 'PUT',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify(updateData)
			});
			
			if (!response.ok) {
				throw new Error(`API error: ${response.status} ${response.statusText}`);
			}
			
			await loadRules();
			dispatch('refresh');
		} catch (err) {
			alert(`Failed to update rule status: ${err.message}`);
		}
	}

	// Modal handlers
	function openRuleEditor(rule?: AllowlistRule) {
		editingRule = rule || null;
		showRuleEditor = true;
	}

	function closeRuleEditor() {
		showRuleEditor = false;
		editingRule = null;
	}

	async function handleRuleSave(event: CustomEvent<CreateAllowlistRule>) {
		try {
			const ruleData = event.detail;
			
			// Map frontend fields to backend fields
			const backendData = {
				name: ruleData.name,
				pattern: ruleData.pattern,
				action: ruleData.action,
				priority: ruleData.priority || 100,
				enabled: ruleData.active ?? true,
				// Map type to backend format if needed
				rule_type: ruleData.type || 'tool'
			};
			
			let response;
			if (editingRule) {
				// Update existing rule
				response = await fetch(`/api/security/allowlist/rules/${editingRule.id}`, {
					method: 'PUT',
					headers: {
						'Content-Type': 'application/json'
					},
					body: JSON.stringify(backendData)
				});
			} else {
				// Create new rule
				response = await fetch('/api/security/allowlist/rules', {
					method: 'POST',
					headers: {
						'Content-Type': 'application/json'
					},
					body: JSON.stringify(backendData)
				});
			}
			
			if (!response.ok) {
				const errorData = await response.json().catch(() => ({}));
				throw new Error(`API error: ${response.status} ${response.statusText} - ${errorData.message || ''}`);
			}
			
			await loadRules();
			closeRuleEditor();
			dispatch('refresh');
		} catch (err) {
			alert(`Failed to save rule: ${err.message}`);
		}
	}

	// Selection management
	function toggleRuleSelection(ruleId: string) {
		const newSelected = new Set(selectedRules);
		if (newSelected.has(ruleId)) {
			newSelected.delete(ruleId);
		} else {
			newSelected.add(ruleId);
		}
		selectedRules = newSelected;
		showBulkActions = selectedRules.size > 0;
	}

	function selectAllRules() {
		selectedRules = new Set(filteredRules.map(r => r.id));
		showBulkActions = true;
	}

	function clearSelection() {
		selectedRules = new Set();
		showBulkActions = false;
	}

	// Bulk operations
	async function performBulkOperation(operation: 'enable' | 'disable' | 'delete') {
		if (selectedRules.size === 0) return;

		const ruleIds = Array.from(selectedRules);
		const confirmMessage = 
			operation === 'delete' 
				? `Are you sure you want to delete ${ruleIds.length} rules? This action cannot be undone.`
				: `Are you sure you want to ${operation} ${ruleIds.length} rules?`;

		if (!confirm(confirmMessage)) return;

		try {
			bulkOperationInProgress = true;
			let successCount = 0;
			let failures = [];
			
			// Process rules individually since we don't have a bulk API yet
			for (const ruleId of ruleIds) {
				try {
					if (operation === 'delete') {
						const response = await fetch(`/api/security/allowlist/rules/${ruleId}`, {
							method: 'DELETE'
						});
						if (!response.ok) throw new Error(`Delete failed: ${response.statusText}`);
					} else {
						const enabled = operation === 'enable';
						const response = await fetch(`/api/security/allowlist/rules/${ruleId}`, {
							method: 'PUT',
							headers: { 'Content-Type': 'application/json' },
							body: JSON.stringify({ enabled })
						});
						if (!response.ok) throw new Error(`Update failed: ${response.statusText}`);
					}
					successCount++;
				} catch (err) {
					failures.push(`Rule ${ruleId}: ${err.message}`);
				}
			}
			
			if (failures.length > 0) {
				alert(`Bulk operation completed with ${failures.length} failures:\n${failures.join('\n')}`);
			}
			
			await loadRules();
			clearSelection();
			dispatch('refresh');
		} catch (err) {
			alert(`Bulk operation failed: ${err.message}`);
		} finally {
			bulkOperationInProgress = false;
		}
	}

	// Pagination
	function changePage(page: number) {
		currentPage = Math.max(1, Math.min(page, totalPages));
	}

	onMount(() => {
		loadRules();
	});
</script>

<div class="space-y-6">
	<!-- Header Section -->
	<div class="security-card">
		<div class="security-card-header">
			<div>
				<h2 class="security-card-title">🔧 Allowlist Rule Management</h2>
				<p class="text-sm text-gray-600 mt-1">
					Control which tools and resources can be accessed by users and API keys
				</p>
			</div>
			
			<div class="flex items-center gap-3">
				<button 
					class="btn-secondary"
					on:click={() => showRuleTester = true}
				>
					🧪 Test Rules
				</button>
				
				<button 
					class="btn-primary"
					on:click={() => openRuleEditor()}
				>
					➕ Add Rule
				</button>
			</div>
		</div>

		<!-- Statistics Cards -->
		<div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mt-6">
			<div class="bg-gray-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-gray-900">{ruleStats.total}</div>
				<div class="text-sm text-gray-600">Total Rules</div>
			</div>
			
			<div class="bg-green-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-green-700">{ruleStats.active}</div>
				<div class="text-sm text-green-600">Active</div>
			</div>
			
			<div class="bg-gray-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-gray-600">{ruleStats.inactive}</div>
				<div class="text-sm text-gray-600">Inactive</div>
			</div>
			
			<div class="bg-blue-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-blue-700">{ruleStats.byType.tool}</div>
				<div class="text-sm text-blue-600">Tool Rules</div>
			</div>
			
			<div class="bg-purple-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-purple-700">{ruleStats.byType.resource}</div>
				<div class="text-sm text-purple-600">Resource Rules</div>
			</div>
			
			<div class="bg-orange-50 p-4 rounded-lg">
				<div class="text-2xl font-bold text-orange-700">{ruleStats.byType.global}</div>
				<div class="text-sm text-orange-600">Global Rules</div>
			</div>
		</div>
	</div>

	<!-- Filters and Search -->
	<div class="security-card">
		<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
			<!-- Search -->
			<div class="lg:col-span-2">
				<label class="block text-sm font-medium text-gray-700 mb-2">Search Rules</label>
				<input
					type="text"
					bind:value={searchQuery}
					placeholder="Search by name or pattern..."
					class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
				/>
			</div>

			<!-- Type Filter -->
			<div>
				<label class="block text-sm font-medium text-gray-700 mb-2">Type</label>
				<select bind:value={filterType} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
					<option value="all">All Types</option>
					<option value="tool">Tool Rules</option>
					<option value="resource">Resource Rules</option>
					<option value="global">Global Rules</option>
				</select>
			</div>

			<!-- Action Filter -->
			<div>
				<label class="block text-sm font-medium text-gray-700 mb-2">Action</label>
				<select bind:value={filterAction} class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
					<option value="all">All Actions</option>
					<option value="allow">Allow</option>
					<option value="deny">Deny</option>
				</select>
			</div>

			<!-- Sort Options -->
			<div>
				<label class="block text-sm font-medium text-gray-700 mb-2">Sort By</label>
				<div class="flex gap-2">
					<select bind:value={sortBy} class="flex-1 px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500">
						<option value="priority">Priority</option>
						<option value="name">Name</option>
						<option value="created">Created</option>
						<option value="modified">Modified</option>
					</select>
					
					<button
						class="px-3 py-2 border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500"
						on:click={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'}
					>
						{sortOrder === 'asc' ? '↑' : '↓'}
					</button>
				</div>
			</div>
		</div>
	</div>

	<!-- Bulk Actions Bar -->
	{#if showBulkActions}
		<div class="security-card bg-blue-50 border-blue-200">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-4">
					<span class="text-sm font-medium text-blue-900">
						{selectedRules.size} rules selected
					</span>
					
					<div class="flex items-center gap-2">
						<button
							class="text-sm text-blue-700 hover:text-blue-900 underline"
							on:click={selectAllRules}
						>
							Select All ({filteredRules.length})
						</button>
						
						<span class="text-blue-500">|</span>
						
						<button
							class="text-sm text-blue-700 hover:text-blue-900 underline"
							on:click={clearSelection}
						>
							Clear Selection
						</button>
					</div>
				</div>

				<div class="flex items-center gap-2">
					<button
						class="btn-sm btn-secondary"
						disabled={bulkOperationInProgress}
						on:click={() => performBulkOperation('enable')}
					>
						✅ Enable
					</button>
					
					<button
						class="btn-sm btn-secondary"
						disabled={bulkOperationInProgress}
						on:click={() => performBulkOperation('disable')}
					>
						⏸️ Disable
					</button>
					
					<button
						class="btn-sm btn-danger"
						disabled={bulkOperationInProgress}
						on:click={() => performBulkOperation('delete')}
					>
						🗑️ Delete
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Rules List -->
	<div class="space-y-4">
		{#if loading}
			<div class="security-card">
				<div class="flex items-center justify-center py-12">
					<div class="flex items-center gap-3 text-gray-600">
						<div class="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-600"></div>
						<span>Loading allowlist rules...</span>
					</div>
				</div>
			</div>
		{:else if error}
			<div class="security-card">
				<div class="text-center py-12">
					<div class="text-red-600 mb-4">❌</div>
					<h3 class="text-lg font-medium text-gray-900 mb-2">Failed to Load Rules</h3>
					<p class="text-gray-600 mb-4">{error}</p>
					<button class="btn-primary" on:click={loadRules}>
						🔄 Retry
					</button>
				</div>
			</div>
		{:else if paginatedRules.length === 0}
			<div class="security-card">
				<div class="text-center py-12">
					<div class="text-gray-400 mb-4 text-4xl">📋</div>
					<h3 class="text-lg font-medium text-gray-900 mb-2">
						{searchQuery || filterType !== 'all' || filterAction !== 'all' 
							? 'No Rules Match Your Filters' 
							: 'No Allowlist Rules'}
					</h3>
					<p class="text-gray-600 mb-4">
						{searchQuery || filterType !== 'all' || filterAction !== 'all'
							? 'Try adjusting your search criteria or filters'
							: 'Get started by creating your first allowlist rule'}
					</p>
					<button class="btn-primary" on:click={() => openRuleEditor()}>
						➕ Create First Rule
					</button>
				</div>
			</div>
		{:else}
			{#each paginatedRules as rule}
				<AllowlistRuleCard
					{rule}
					selected={selectedRules.has(rule.id)}
					on:select={() => toggleRuleSelection(rule.id)}
					on:edit={() => openRuleEditor(rule)}
					on:delete={() => deleteRule(rule.id)}
					on:toggle={() => toggleRuleStatus(rule)}
				/>
			{/each}

			<!-- Pagination -->
			{#if totalPages > 1}
				<div class="security-card">
					<div class="flex items-center justify-between">
						<div class="text-sm text-gray-600">
							Showing {((currentPage - 1) * itemsPerPage) + 1} to {Math.min(currentPage * itemsPerPage, filteredRules.length)} of {filteredRules.length} rules
						</div>

						<div class="flex items-center gap-2">
							<button
								class="btn-sm btn-secondary"
								disabled={currentPage <= 1}
								on:click={() => changePage(currentPage - 1)}
							>
								← Previous
							</button>

							{#each Array.from({length: Math.min(5, totalPages)}, (_, i) => i + Math.max(1, currentPage - 2)) as page}
								{#if page <= totalPages}
									<button
										class="btn-sm {page === currentPage ? 'btn-primary' : 'btn-secondary'}"
										on:click={() => changePage(page)}
									>
										{page}
									</button>
								{/if}
							{/each}

							<button
								class="btn-sm btn-secondary"
								disabled={currentPage >= totalPages}
								on:click={() => changePage(currentPage + 1)}
							>
								Next →
							</button>
						</div>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>

<!-- Rule Editor Modal -->
{#if showRuleEditor}
	<AllowlistRuleEditor
		rule={editingRule}
		on:save={handleRuleSave}
		on:cancel={closeRuleEditor}
	/>
{/if}

<!-- Rule Tester Modal -->
{#if showRuleTester}
	<AllowlistRuleTester
		{rules}
		on:close={() => showRuleTester = false}
	/>
{/if}

<style>
	/* Use allowlist styling - leveraging security design system */
	.security-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
		box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1);
	}

	.security-card-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.security-card-title {
		font-size: 1.5rem;
		font-weight: 600;
		color: #1f2937;
		margin: 0;
	}

	.btn-primary {
		background-color: #2563eb;
		color: white;
		border: none;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.btn-primary:hover {
		background-color: #1d4ed8;
	}

	.btn-secondary {
		background-color: #6b7280;
		color: white;
		border: none;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.btn-secondary:hover {
		background-color: #4b5563;
	}

	.btn-sm {
		padding: 6px 12px;
		font-size: 0.875rem;
	}

	.btn-danger {
		background-color: #dc2626;
		color: white;
		border: none;
		padding: 8px 16px;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: background-color 0.2s;
	}

	.btn-danger:hover {
		background-color: #b91c1c;
	}

	/* Grid utilities for Tailwind-like layout */
	.space-y-4 > * + * {
		margin-top: 1rem;
	}

	.space-y-6 > * + * {
		margin-top: 1.5rem;
	}

	.grid {
		display: grid;
	}

	.grid-cols-2 {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	.grid-cols-4 {
		grid-template-columns: repeat(4, minmax(0, 1fr));
	}

	.grid-cols-6 {
		grid-template-columns: repeat(6, minmax(0, 1fr));
	}

	.gap-4 {
		gap: 1rem;
	}

	.gap-2 {
		gap: 0.5rem;
	}

	.gap-3 {
		gap: 0.75rem;
	}

	.flex {
		display: flex;
	}

	.items-center {
		align-items: center;
	}

	.justify-between {
		justify-content: space-between;
	}

	.text-sm {
		font-size: 0.875rem;
	}

	.text-lg {
		font-size: 1.125rem;
	}

	.text-2xl {
		font-size: 1.5rem;
	}

	.text-4xl {
		font-size: 2.25rem;
	}

	.font-bold {
		font-weight: 700;
	}

	.font-medium {
		font-weight: 500;
	}

	.text-gray-600 {
		color: #4b5563;
	}

	.text-gray-700 {
		color: #374151;
	}

	.text-gray-900 {
		color: #111827;
	}

	.text-gray-400 {
		color: #9ca3af;
	}

	.text-green-700 {
		color: #15803d;
	}

	.text-green-600 {
		color: #16a34a;
	}

	.text-blue-700 {
		color: #1d4ed8;
	}

	.text-blue-600 {
		color: #2563eb;
	}

	.text-blue-900 {
		color: #1e3a8a;
	}

	.text-blue-500 {
		color: #3b82f6;
	}

	.text-purple-700 {
		color: #7c3aed;
	}

	.text-purple-600 {
		color: #9333ea;
	}

	.text-orange-700 {
		color: #c2410c;
	}

	.text-orange-600 {
		color: #ea580c;
	}

	.text-red-600 {
		color: #dc2626;
	}

	.bg-gray-50 {
		background-color: #f9fafb;
	}

	.bg-green-50 {
		background-color: #f0fdf4;
	}

	.bg-blue-50 {
		background-color: #eff6ff;
	}

	.bg-purple-50 {
		background-color: #faf5ff;
	}

	.bg-orange-50 {
		background-color: #fff7ed;
	}

	.border-blue-200 {
		border-color: #bfdbfe;
	}

	.p-4 {
		padding: 1rem;
	}

	.py-12 {
		padding-top: 3rem;
		padding-bottom: 3rem;
	}

	.px-3 {
		padding-left: 0.75rem;
		padding-right: 0.75rem;
	}

	.py-2 {
		padding-top: 0.5rem;
		padding-bottom: 0.5rem;
	}

	.mb-2 {
		margin-bottom: 0.5rem;
	}

	.mb-4 {
		margin-bottom: 1rem;
	}

	.mt-1 {
		margin-top: 0.25rem;
	}

	.mt-6 {
		margin-top: 1.5rem;
	}

	.w-full {
		width: 100%;
	}

	.w-6 {
		width: 1.5rem;
	}

	.h-6 {
		height: 1.5rem;
	}

	.rounded-lg {
		border-radius: 0.5rem;
	}

	.rounded-md {
		border-radius: 0.375rem;
	}

	.rounded-full {
		border-radius: 9999px;
	}

	.border {
		border-width: 1px;
	}

	.border-gray-300 {
		border-color: #d1d5db;
	}

	.border-b-2 {
		border-bottom-width: 2px;
	}

	.border-blue-600 {
		border-color: #2563eb;
	}

	.text-center {
		text-align: center;
	}

	.underline {
		text-decoration: underline;
	}

	.animate-spin {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.focus\:outline-none:focus {
		outline: 2px solid transparent;
		outline-offset: 2px;
	}

	.focus\:ring-2:focus {
		--tw-ring-offset-shadow: var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color);
		--tw-ring-shadow: var(--tw-ring-inset) 0 0 0 calc(2px + var(--tw-ring-offset-width)) var(--tw-ring-color);
		box-shadow: var(--tw-ring-offset-shadow), var(--tw-ring-shadow);
	}

	.focus\:ring-blue-500:focus {
		--tw-ring-color: #3b82f6;
	}

	.hover\:bg-gray-50:hover {
		background-color: #f9fafb;
	}

	.hover\:text-blue-900:hover {
		color: #1e3a8a;
	}

	/* Responsive design */
	@media (min-width: 768px) {
		.md\:grid-cols-2 {
			grid-template-columns: repeat(2, minmax(0, 1fr));
		}

		.md\:grid-cols-4 {
			grid-template-columns: repeat(4, minmax(0, 1fr));
		}
	}

	@media (min-width: 1024px) {
		.lg\:grid-cols-6 {
			grid-template-columns: repeat(6, minmax(0, 1fr));
		}

		.lg\:grid-cols-5 {
			grid-template-columns: repeat(5, minmax(0, 1fr));
		}

		.lg\:col-span-2 {
			grid-column: span 2 / span 2;
		}
	}

	/* Custom component styles */
	.flex-1 {
		flex: 1 1 0%;
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	button:disabled:hover {
		background-color: initial;
	}
</style>