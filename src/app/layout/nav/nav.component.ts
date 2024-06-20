import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-nav',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    RouterLink,
    RouterLinkActive
  ],
  templateUrl: './nav.component.html',
  styleUrl: './nav.component.css'
})
export class NavComponent {
  // Toogle
  currentSection: string = '';
  showMe: boolean = true;
  visible: boolean = false;
  toggle(section: string) {
    if (this.currentSection === section) {
      this.currentSection = ''; // Si se hace clic en la misma sección, la ocultamos
    } else {
      this.currentSection = section; // Si se hace clic en una nueva sección, la mostramos
    }
  }

}
