import {Component, EventEmitter, Input, OnDestroy, OnInit, Output, ViewChild, ElementRef, AfterViewInit, ChangeDetectorRef, NgZone} from "@angular/core";
import {IDropdownSettings} from "ng-multiselect-dropdown/multiselect.model";
import {AdditionalButton} from "../../domain_value/additional_button";
import set = Reflect.set;
import {Subscription} from "rxjs";
import {delay} from "rxjs/operators";

@Component({
    selector: "MultiSelect",
    templateUrl: "./multi_select.html",
    styleUrls: ["./multi_select.scss"]
})
export class MultiSelectComponent implements OnInit, OnDestroy, AfterViewInit {
    constructor(private cdr: ChangeDetectorRef, private ngZone: NgZone) {}

    @ViewChild("child", {static: true, read: ElementRef}) elementRef: ElementRef;

    @Input()
    placeholder: string = 'Placeholder';

    dropdownListData = [];

    @Input()
    set dropdownList(list: Array<any>) {
        this.dropdownListData = list;
        this.scheduleButtonUpdate();
    }

    @Input()
    enableCheckAll: boolean = false;
    @Input()
    allowSearchFilter: boolean = true;
    @Input()
    dropdownSettings: IDropdownSettings = {
        idField: 'id',
        textField: 'label',
        selectAllText: 'Select all',
        unSelectAllText: 'Deselect all',
        itemsShowLimit: 1,
    };

    additional_buttonData: Array<AdditionalButton> = [];
    @Input()
    set additional_button(buttons: Array<AdditionalButton>) {
        this.additional_buttonData = buttons;
        this.scheduleButtonUpdate();
    }

    selectedItemsData: Array<any> = [];

    @Input()
    get selectedItems(): Array<any> {
        return this.selectedItemsData;
    }

    set selectedItems(items: Array<any>) {
        this.selectedItemsData = items;
        this.selectedItemsChange.emit(items);
        this.check_if_additional_buttons_are_selected();
    }

    @Output()
    items_changed_by_action: EventEmitter<void> = new EventEmitter();

    @Output()
    item_selected: EventEmitter<any> = new EventEmitter<any>();
    @Output()
    item_deselected: EventEmitter<any> = new EventEmitter<any>();
    @Output()
    select_all: EventEmitter<any> = new EventEmitter<any>();
    @Output()
    deselect_all: EventEmitter<any> = new EventEmitter<any>();
    @Output()
    selectedItemsChange: EventEmitter<Array<any>> = new EventEmitter();

    private additional_button_checkboxes: Map<number, any> = new Map();
    private subscription: Subscription = new Subscription();
    private buttonUpdatePending = false;
    private mutationObserver: MutationObserver;
    private isInitialized = false;

    ngOnInit(): void {
        this.dropdownSettings.enableCheckAll = this.enableCheckAll;
        this.dropdownSettings.allowSearchFilter = this.allowSearchFilter;
        this.subscription.add(this.item_selected.subscribe(() => this.items_changed_by_action.next()));
        this.subscription.add(this.item_deselected.subscribe(() => this.items_changed_by_action.next()));
        this.subscription.add(this.select_all.pipe(delay(500)).subscribe(() => this.items_changed_by_action.next()));
        this.subscription.add(this.deselect_all.pipe(delay(500)).subscribe(() => this.items_changed_by_action.next()));
    }

    ngAfterViewInit(): void {
        this.isInitialized = true;
        this.setupMutationObserver();
        this.scheduleButtonUpdate();
    }

    ngOnDestroy(): void {
        this.subscription?.unsubscribe();
        if (this.mutationObserver) {
            this.mutationObserver.disconnect();
        }
    }

    private scheduleButtonUpdate(): void {
        if (!this.isInitialized || this.buttonUpdatePending) {
            return;
        }
        
        this.buttonUpdatePending = true;
        this.ngZone.runOutsideAngular(() => {
            setTimeout(() => {
                this.ngZone.run(() => {
                    this.addCollectionButton();
                    this.buttonUpdatePending = false;
                });
            }, 100);
        });
    }

    private setupMutationObserver(): void {
        const targetNode = this.elementRef.nativeElement;
        
        this.mutationObserver = new MutationObserver((mutations) => {
            let shouldUpdate = false;
            mutations.forEach((mutation) => {
                if (mutation.type === 'childList') {
                    const dropdownList = targetNode.querySelector('.dropdown-list');
                    if (dropdownList && !this.hasAdditionalButtons(dropdownList)) {
                        shouldUpdate = true;
                    }
                }
            });
            
            if (shouldUpdate) {
                this.scheduleButtonUpdate();
            }
        });
        
        this.mutationObserver.observe(targetNode, {
            childList: true,
            subtree: true
        });
    }

    private hasAdditionalButtons(dropdownList: Element): boolean {
        const item1 = dropdownList.querySelector('.item1');
        if (!item1 || this.additional_buttonData.length === 0) {
            return true;
        }
        
        return this.additional_buttonData.every(button => 
            item1.querySelector(`#additional_button${button.id}`) !== null
        );
    }

    private addCollectionButton(): void {
        const root = this.elementRef.nativeElement.querySelector('.dropdown-list');
        if (!root) {
            return;
        }
        
        const collection_button_ul = root.querySelector('.item1');
        if (!collection_button_ul) {
            return;
        }
       
        const ul_copy = collection_button_ul.cloneNode(false); 
        const segments = root.querySelector('.item2');

        if (this.hasAdditionalButtons(root)) {
            return;
        }

        this.additional_button_checkboxes.clear();
        if (collection_button_ul.children.length > 1) {
            const select_all_checkbox = collection_button_ul.children[0];
            select_all_checkbox.style.padding = "6px 10px";
            select_all_checkbox.style.borderBottom = "none";
            const search_textbox = collection_button_ul.children[collection_button_ul.children.length - 1];
            
            const elementsToKeep = [select_all_checkbox];
            const searchTextboxToMove = search_textbox.className.includes("filter-textbox") ? search_textbox : null;
            
            collection_button_ul.innerHTML = "";

            collection_button_ul.appendChild(select_all_checkbox);
            for (const button of this.additional_buttonData) {
                const clone = select_all_checkbox.cloneNode(true);
                clone.children[1].innerHTML = button.label;
                clone.children[1].id = "additional_button" + button.id.toString();
                clone.children[0].checked = false;
                clone.addEventListener("click", () => {
                    clone.children[0].checked = !clone.children[0].checked;
                    this.selectedItems = button.list_selection_callback(button, this.selectedItemsData, this.dropdownListData, clone.children[0].checked);
                    this.items_changed_by_action.next();
                });
                this.additional_button_checkboxes.set(button.id, clone.children[0]);
                collection_button_ul.appendChild(clone);
            }
            
            if (collection_button_ul.children.length > 0) {
                collection_button_ul.children[collection_button_ul.children.length - 1].style.paddingBottom = "12px";
            }
            
            if (searchTextboxToMove) {
                searchTextboxToMove.style.borderBottom = "1px solid #ccc";
                searchTextboxToMove.style.borderTop = "1px solid #ccc";
                searchTextboxToMove.style.paddingBottom = "12px";
                ul_copy.appendChild(searchTextboxToMove);
                if (segments) {
                    root.insertBefore(ul_copy, segments);
                }
            }
            this.check_if_additional_buttons_are_selected();
        }
    }

    private check_if_additional_buttons_are_selected(): void {
        for (const button of this.additional_buttonData) {
            if (!this.additional_button_checkboxes.has(button.id))
                continue;

            const required_items = button.list_selection_callback(button, this.selectedItemsData, this.dropdownListData, true);
            const checked = required_items.every(item => this.selectedItemsData.find(r_item => r_item.id === item.id) !== undefined);
            this.additional_button_checkboxes.get(button.id).checked = checked;
        }
    }
}
