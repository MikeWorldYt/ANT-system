import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class MenuService {
  private menuSource = new BehaviorSubject<boolean>(true);
  currentMenu = this.menuSource.asObservable();

  constructor() { }

  changeMenu(menu: boolean) {
    this.menuSource.next(menu);
  }
}